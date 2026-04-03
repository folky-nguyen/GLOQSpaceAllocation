use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::layout_contract::*;

pub fn phase_for_override_key(key: &str) -> SolvePhase {
    if key.starts_with("jurisdiction_profile")
        || key.starts_with("building_shape")
        || key.starts_with("building_construction_type")
        || key.starts_with("site_polygon")
        || key.starts_with("site_planning.")
        || key.starts_with("code_profile_overrides.")
        || key.starts_with("solver_controls.")
    {
        SolvePhase::InputNormalization
    } else if key.starts_with("levels.")
        || key.starts_with("targets.")
        || key.starts_with("shape_parameters.")
    {
        SolvePhase::MassingTargets
    } else if key.starts_with("unit_mix.")
        || key.starts_with("amenities.")
        || key.starts_with("optimization.")
        || key.starts_with("residential_features.")
    {
        SolvePhase::ProgramTargets
    } else if key.starts_with("vertical_rules.") {
        SolvePhase::VerticalSystem
    } else if key.starts_with("constraints.") {
        SolvePhase::ProgramTargets
    } else {
        SolvePhase::ProgramTargets
    }
}

pub fn solve_input_normalization(state: &mut EngineState) {
    let mut normalized = NormalizedInput {
        jurisdiction_profile: state.input.jurisdiction_profile.clone(),
        building_shape: state.input.building_shape,
        building_construction_type: state.input.building_construction_type,
        site_polygon: state.input.site_polygon.clone(),
        site_planning: state.input.site_planning.clone(),
        levels: state.input.levels.clone(),
        unit_mix_seed: state.input.unit_mix.clone().normalized(),
        constraints: state.input.constraints.clone(),
        residential_features: state.input.residential_features.clone(),
        vertical_rules: state.input.vertical_rules.clone(),
        amenities: state.input.amenities.clone(),
        targets: state.input.targets.clone(),
        optimization: state.input.optimization.clone(),
        code_profile_overrides: state.input.code_profile_overrides.clone(),
        shape_parameters: state.input.shape_parameters.clone(),
        solver_controls: state.input.solver_controls.clone(),
    };

    state.variables.insert(
        "input.site_area_seed",
        SolvePhase::InputNormalization,
        ValueSource::Formula,
        Some(ScalarValue::F64(polygon_area_sf(&normalized.site_polygon))),
        None,
        ScalarValue::F64(polygon_area_sf(&normalized.site_polygon)),
        Some("sf"),
        &["site_polygon"],
    );

    let mut earliest_dirty = SolvePhase::OutputAssembly;
    for ov in &state.input.user_overrides {
        if !ov.apply {
            continue;
        }
        let phase = phase_for_override_key(&ov.key);
        if phase < earliest_dirty {
            earliest_dirty = phase;
        }
        apply_override_to_normalized_input(&mut normalized, ov, &mut state.variables);
    }

    state.variables.insert(
        "unit_mix.seed_sum",
        SolvePhase::InputNormalization,
        ValueSource::Formula,
        Some(ScalarValue::F64(normalized.unit_mix_seed.sum())),
        None,
        ScalarValue::F64(normalized.unit_mix_seed.sum()),
        None,
        &[
            "unit_mix.studio",
            "unit_mix.one_bedroom",
            "unit_mix.two_bedroom",
            "unit_mix.three_bedroom",
        ],
    );

    state.normalized = Some(normalized);

    if state
        .normalized
        .as_ref()
        .map(|input| {
            input.site_planning.california_mode && input.site_planning.overlay.overlay_id.is_none()
        })
        .unwrap_or(false)
    {
        state.validation_issues.push(ValidationIssue::warning(
            "local_overlay_missing_or_assumed",
            "California site-planning mode is active without an explicit local overlay binding; exterior numerics will use configurable fallback assumptions.",
        ));
    }

    if earliest_dirty != SolvePhase::OutputAssembly {
        state.validation_issues.push(ValidationIssue::warning(
            "override_recompute_triggered",
            "user override detected; engine recomputed downstream phases from the earliest affected phase",
        ));
    }
}

pub fn solve_massing_targets(state: &mut EngineState) {
    let input = state.normalized.as_ref().unwrap();
    let massing = solve_massing_for_shape(input, &state.assumptions, input.building_shape, None);
    let gfa_goal = massing.gfa_goal_sf;
    let story_count = massing.story_count;
    state.massing = Some(massing);

    state.variables.insert(
        "targets.gfa_goal_sf",
        SolvePhase::MassingTargets,
        ValueSource::Formula,
        Some(ScalarValue::F64(gfa_goal)),
        None,
        ScalarValue::F64(gfa_goal),
        Some("sf"),
        &["targets.far_max", "optimization.far_fill_target"],
    );
    state.variables.insert(
        "levels.story_count_resolved",
        SolvePhase::MassingTargets,
        ValueSource::Formula,
        Some(ScalarValue::U32(story_count)),
        None,
        ScalarValue::U32(story_count),
        Some("story"),
        &["levels.count", "optimization.allow_story_override"],
    );
}

/* =========================== construction / shape ========================= */

pub fn classify_construction_case(t: BuildingConstructionType) -> ConstructionCase {
    match t {
        BuildingConstructionType::TypeV => ConstructionCase::LowRiseTypeV,
        BuildingConstructionType::TypeIII => ConstructionCase::MidRiseTypeIII,
        BuildingConstructionType::TypeVOverI => ConstructionCase::PodiumTypeVOverI,
        BuildingConstructionType::TypeIIIOverI => ConstructionCase::PodiumTypeIIIOverI,
        BuildingConstructionType::TypeI => ConstructionCase::HighRiseTypeI,
    }
}

pub fn classify_shape_case(shape: BuildingShape) -> ShapeCase {
    match shape {
        BuildingShape::Bar | BuildingShape::Tower => ShapeCase::Linear,
        BuildingShape::LShape
        | BuildingShape::UShape
        | BuildingShape::OShape
        | BuildingShape::PerimeterPartial => ShapeCase::Courtyard,
        BuildingShape::HShape | BuildingShape::XShape => ShapeCase::Branched,
        BuildingShape::Cluster | BuildingShape::FreeForm => ShapeCase::Distributed,
    }
}

pub fn default_corridor_candidates(input: &NormalizedInput) -> Vec<CorridorType> {
    match input.constraints.corridor_type {
        CorridorType::Auto => vec![
            CorridorType::DoubleLoaded,
            CorridorType::SingleLoaded,
            CorridorType::Perimeter,
            CorridorType::Internal,
        ],
        x => vec![x],
    }
}

pub fn default_core_candidates(input: &NormalizedInput) -> Vec<CoreStrategy> {
    match input.constraints.core_strategy {
        CoreStrategy::Auto => vec![
            CoreStrategy::Central,
            CoreStrategy::Corner,
            CoreStrategy::Multiple,
            CoreStrategy::Distributed,
        ],
        x => vec![x],
    }
}

fn all_building_shapes() -> Vec<BuildingShape> {
    vec![
        BuildingShape::Bar,
        BuildingShape::LShape,
        BuildingShape::UShape,
        BuildingShape::OShape,
        BuildingShape::HShape,
        BuildingShape::Tower,
        BuildingShape::XShape,
        BuildingShape::Cluster,
        BuildingShape::FreeForm,
        BuildingShape::PerimeterPartial,
    ]
}

fn dedup_sorted_f64(values: &mut Vec<f64>) {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    values.dedup_by(|a, b| (*a - *b).abs() <= EPS);
}

fn point_on_segment(point: Point2, a: Point2, b: Point2) -> bool {
    let cross = (point.y - a.y) * (b.x - a.x) - (point.x - a.x) * (b.y - a.y);
    if cross.abs() > 1.0e-6 {
        return false;
    }
    let dot = (point.x - a.x) * (point.x - b.x) + (point.y - a.y) * (point.y - b.y);
    dot <= 1.0e-6
}

fn point_in_polygon(point: Point2, polygon: &[Point2]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        if point_on_segment(point, a, b) {
            return true;
        }
        let intersects = ((a.y > point.y) != (b.y > point.y))
            && (point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y + EPS) + a.x);
        if intersects {
            inside = !inside;
        }
    }
    inside
}

fn merge_occupied_cells_to_rects(
    xs: &[f64],
    ys: &[f64],
    occupied: &BTreeSet<(usize, usize)>,
) -> Vec<Rect2> {
    let mut strips = Vec::<(usize, usize, usize, usize)>::new();
    for yi in 0..ys.len().saturating_sub(1) {
        let mut xi = 0usize;
        while xi < xs.len().saturating_sub(1) {
            if !occupied.contains(&(xi, yi)) {
                xi += 1;
                continue;
            }
            let start = xi;
            while xi < xs.len().saturating_sub(1) && occupied.contains(&(xi, yi)) {
                xi += 1;
            }
            let end = xi;
            if let Some(last) = strips.last_mut() {
                if last.0 == start && last.1 == end && last.3 == yi {
                    last.3 = yi + 1;
                    continue;
                }
            }
            strips.push((start, end, yi, yi + 1));
        }
    }

    strips
        .into_iter()
        .map(|(x0, x1, y0, y1)| Rect2::new(xs[x0], ys[y0], xs[x1], ys[y1]))
        .filter(|rect| rect.area() > EPS)
        .collect()
}

fn clip_rect_to_site_polygon(rect: Rect2, site_poly: &[Point2]) -> Vec<Rect2> {
    if rect.area() <= EPS || site_poly.len() < 3 {
        return Vec::new();
    }
    let site_bbox = bounding_rect(site_poly);
    let clipped = Rect2::new(
        rect.min_x.max(site_bbox.min_x),
        rect.min_y.max(site_bbox.min_y),
        rect.max_x.min(site_bbox.max_x),
        rect.max_y.min(site_bbox.max_y),
    );
    if clipped.area() <= EPS {
        return Vec::new();
    }

    let mut xs = site_poly
        .iter()
        .map(|p| p.x)
        .chain([clipped.min_x, clipped.max_x])
        .filter(|x| *x >= clipped.min_x - EPS && *x <= clipped.max_x + EPS)
        .collect::<Vec<_>>();
    let mut ys = site_poly
        .iter()
        .map(|p| p.y)
        .chain([clipped.min_y, clipped.max_y])
        .filter(|y| *y >= clipped.min_y - EPS && *y <= clipped.max_y + EPS)
        .collect::<Vec<_>>();
    dedup_sorted_f64(&mut xs);
    dedup_sorted_f64(&mut ys);
    if xs.len() < 2 || ys.len() < 2 {
        return vec![clipped];
    }

    let mut occupied = BTreeSet::<(usize, usize)>::new();
    for xi in 0..xs.len().saturating_sub(1) {
        for yi in 0..ys.len().saturating_sub(1) {
            let probe = Point2::new((xs[xi] + xs[xi + 1]) * 0.5, (ys[yi] + ys[yi + 1]) * 0.5);
            if clipped.contains(probe) && point_in_polygon(probe, site_poly) {
                occupied.insert((xi, yi));
            }
        }
    }

    let realized = merge_occupied_cells_to_rects(&xs, &ys, &occupied);
    if realized.is_empty() {
        vec![clipped]
    } else {
        realized
    }
}

fn realize_shape_rects(
    shape: BuildingShape,
    target_area_sf: f64,
    site_poly: &[Point2],
    a: &AssumptionPack,
) -> (Vec<Rect2>, ShapeRealizationDiagnostics) {
    let seed_rects = stable_block_rects(&seed_shape_rects(shape, target_area_sf, site_poly, a));
    let mut realized = Vec::<Rect2>::new();
    for rect in &seed_rects {
        realized.extend(clip_rect_to_site_polygon(*rect, site_poly));
    }
    realized = stable_block_rects(&realized);
    let seed_area_sf = rects_area_sum(&seed_rects);
    let realized_area_sf = rects_area_sum(&realized);
    let clipping_loss_ratio = if seed_area_sf <= EPS {
        0.0
    } else {
        ((seed_area_sf - realized_area_sf) / seed_area_sf).max(0.0)
    };
    let parcel_fit_ratio = if seed_area_sf <= EPS {
        0.0
    } else {
        realized_area_sf / seed_area_sf
    };
    let realized_frontage_ft = realized
        .iter()
        .map(|rect| 2.0 * (rect.width() + rect.height()))
        .sum::<f64>();
    let mut warnings = Vec::<String>::new();
    if clipping_loss_ratio > 0.18 {
        warnings.push("shape_clipping_loss_high".to_string());
    }
    if realized.len() > seed_rects.len() + 3 {
        warnings.push("shape_fragmentation_high".to_string());
    }
    (
        if realized.is_empty() {
            seed_rects.clone()
        } else {
            realized.clone()
        },
        ShapeRealizationDiagnostics {
            seed_area_sf,
            realized_area_sf: if realized.is_empty() {
                seed_area_sf
            } else {
                realized_area_sf
            },
            realized_frontage_ft,
            clipping_loss_ratio,
            parcel_fit_ratio: if realized.is_empty() {
                1.0
            } else {
                parcel_fit_ratio
            },
            fragment_count: if realized.is_empty() {
                seed_rects.len()
            } else {
                realized.len()
            },
            warnings,
        },
    )
}

fn shape_search_enabled(input: &NormalizedInput) -> bool {
    input
        .solver_controls
        .as_ref()
        .and_then(|ctrl| ctrl.shape_search_enabled)
        .unwrap_or(true)
}

fn prune_incompatible_candidates(input: &NormalizedInput) -> bool {
    input
        .solver_controls
        .as_ref()
        .and_then(|ctrl| ctrl.prune_incompatible_candidates)
        .unwrap_or(true)
}

fn derive_shape_candidates(input: &NormalizedInput) -> Vec<BuildingShape> {
    if !shape_search_enabled(input) {
        return vec![input.building_shape];
    }

    let bbox = bounding_rect(&input.site_polygon);
    let aspect = bbox.width().max(bbox.height()) / bbox.width().min(bbox.height()).max(1.0);
    let vertex_bonus = usize::from(input.site_polygon.len() >= 6) as f64 * 0.12;
    let mut scored = all_building_shapes()
        .into_iter()
        .enumerate()
        .map(|(ordinal, shape)| {
            let mut score = if shape == input.building_shape {
                0.30
            } else {
                0.0
            };
            score += match shape {
                BuildingShape::Bar if aspect >= 1.45 => 0.32,
                BuildingShape::Tower if aspect <= 1.22 => 0.28,
                BuildingShape::UShape | BuildingShape::OShape | BuildingShape::PerimeterPartial
                    if aspect <= 2.25 =>
                {
                    0.22 + vertex_bonus
                }
                BuildingShape::HShape | BuildingShape::XShape | BuildingShape::Cluster => {
                    0.16 + vertex_bonus
                }
                BuildingShape::FreeForm => 0.10 + vertex_bonus,
                BuildingShape::LShape => 0.18,
                _ => 0.06,
            };
            score += match input.building_construction_type {
                BuildingConstructionType::TypeI
                    if matches!(shape, BuildingShape::Tower | BuildingShape::OShape) =>
                {
                    0.12
                }
                BuildingConstructionType::TypeV
                    if matches!(shape, BuildingShape::Bar | BuildingShape::LShape) =>
                {
                    0.08
                }
                _ => 0.0,
            };
            (shape, score, ordinal)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.2.cmp(&b.2))
    });

    let limit = input
        .solver_controls
        .as_ref()
        .and_then(|ctrl| ctrl.shape_search_limit)
        .unwrap_or(5)
        .max(1);
    let mut selected = scored
        .into_iter()
        .take(limit)
        .map(|(shape, _, _)| shape)
        .collect::<Vec<_>>();
    if !selected.iter().any(|shape| *shape == input.building_shape) {
        if selected.len() >= limit {
            selected.pop();
        }
        selected.push(input.building_shape);
    }
    selected.sort_by_key(|shape| {
        all_building_shapes()
            .iter()
            .position(|candidate| candidate == shape)
            .unwrap_or(usize::MAX)
    });
    selected
}

fn select_total_story_count_for_shape(
    input: &NormalizedInput,
    site_area_sf: f64,
    plate_seed_sf: f64,
    shape: BuildingShape,
    shape_diag: &ShapeRealizationDiagnostics,
    a: &AssumptionPack,
) -> u32 {
    let min_total = input
        .optimization
        .story_search_min
        .unwrap_or(input.levels.count)
        .max(input.levels.below_grade_count + input.levels.podium_levels + 1)
        .max(1);
    let max_total = input
        .optimization
        .story_search_max
        .unwrap_or(input.levels.count)
        .max(min_total);

    if !input.optimization.allow_story_override && min_total == max_total {
        return input.levels.count.max(min_total).min(max_total);
    }

    let avg_area = weighted_seed_target_area(input, a);
    let shape_efficiency =
        (0.55 * shape_coverage_ratio(shape) + 0.45 * shape_diag.parcel_fit_ratio).max(0.18);
    let seed_mix = input.unit_mix_seed.clone().normalized();
    let mut best_story_count = input.levels.count.max(min_total).min(max_total);
    let mut best_score = f64::NEG_INFINITY;

    for total_story_count in min_total..=max_total {
        let above_grade = total_story_count
            .saturating_sub(input.levels.below_grade_count)
            .max(1);
        let gfa_est = plate_seed_sf * above_grade as f64;
        let mut du_est = preliminary_dwelling_units_from_gfa(
            gfa_est,
            input.targets.retail_area_sf,
            total_story_count,
            &seed_mix,
            input,
            a,
        )
        .max(1) as f64;
        if let Some(cap) = input.targets.dwelling_units_cap {
            du_est = du_est.min(cap as f64);
        }
        let res_area_est = du_est * avg_area.max(1.0);

        let story_penalty = if total_story_count <= 4 {
            0.04
        } else if total_story_count <= 9 {
            0.02
        } else {
            0.01 * (total_story_count as f64 / 20.0)
        };
        let clipping_penalty = shape_diag.clipping_loss_ratio * 0.20;

        let score = match input.optimization.objective {
            OptimizationObjective::MaximizeFar => {
                shape_efficiency * (gfa_est / site_area_sf.max(1.0))
                    - story_penalty
                    - clipping_penalty
            }
            OptimizationObjective::MaximizeDwellingUnits => {
                shape_efficiency * du_est - total_story_count as f64 * 0.25 - clipping_penalty
            }
            OptimizationObjective::MaximizeBalancedYield => {
                let rent_proxy = du_est * res_area_est.max(1.0).sqrt();
                rent_proxy / 100.0 + 0.30 * shape_efficiency * (gfa_est / site_area_sf.max(1.0))
                    - story_penalty
                    - clipping_penalty
            }
        };

        if score > best_score {
            best_score = score;
            best_story_count = total_story_count;
        }
    }

    best_story_count
}

fn solve_massing_for_shape(
    input: &NormalizedInput,
    a: &AssumptionPack,
    shape: BuildingShape,
    forced_story_count: Option<u32>,
) -> MassingState {
    let site_area = polygon_area_sf(&input.site_polygon);
    let site_perimeter = polygon_perimeter_ft(&input.site_polygon);
    let plate_seed_target = site_area
        * shape_coverage_ratio(shape)
        * construction_multiplier(input.building_construction_type);
    let (_, shape_diagnostics) =
        realize_shape_rects(shape, plate_seed_target, &input.site_polygon, a);
    let plate_seed = shape_diagnostics
        .realized_area_sf
        .max(plate_seed_target * 0.55)
        .min(site_area.max(1.0));

    let story_count = forced_story_count.unwrap_or_else(|| {
        select_total_story_count_for_shape(
            input,
            site_area,
            plate_seed,
            shape,
            &shape_diagnostics,
            a,
        )
    });
    let above_grade_count = story_count
        .saturating_sub(input.levels.below_grade_count)
        .max(1);
    let upper_story_count = above_grade_count
        .saturating_sub(input.levels.podium_levels)
        .max(1);

    let gfa_max_by_far = input.targets.far_max * site_area;
    let gfa_max_by_plate = plate_seed * above_grade_count as f64;
    let gfa_max_by_parking = gfa_max_by_far * parking_gfa_scalar(input.constraints.parking_mode);
    let seed_mix = input.unit_mix_seed.clone().normalized();
    let gfa_max_by_units = input
        .targets
        .dwelling_units_cap
        .map(|cap| {
            preliminary_gfa_from_dwelling_units(
                cap,
                input.targets.retail_area_sf,
                story_count,
                &seed_mix,
                input,
                a,
            )
        })
        .unwrap_or(f64::INFINITY);
    let gfa_goal_unclamped = input.optimization.far_fill_target * gfa_max_by_far;
    let gfa_goal_pre_cap = gfa_goal_unclamped
        .min(gfa_max_by_plate)
        .min(gfa_max_by_parking)
        .min(gfa_max_by_units);
    let gfa_goal = input
        .targets
        .gfa_cap_sf
        .map(|cap| cap.min(gfa_goal_pre_cap))
        .unwrap_or(gfa_goal_pre_cap);
    let preliminary_area_budget = preliminary_area_budget_from_mix(
        gfa_goal,
        input.targets.retail_area_sf,
        story_count,
        &seed_mix,
        input,
        a,
    );

    let podium_footprint_sf = match input.building_construction_type {
        BuildingConstructionType::TypeVOverI | BuildingConstructionType::TypeIIIOverI => {
            (shape_diagnostics.realized_area_sf * 1.08).min(site_area * 0.82)
        }
        _ => 0.0,
    };

    let upper_footprint_sf = if input.levels.podium_levels > 0 {
        ((gfa_goal - input.levels.podium_levels as f64 * podium_footprint_sf)
            / upper_story_count as f64)
            .max(0.0)
            .min(plate_seed)
    } else {
        (gfa_goal / above_grade_count as f64).min(plate_seed)
    };

    let mut massing = MassingState {
        building_shape: shape,
        site_area_sf: site_area,
        site_perimeter_ft: site_perimeter,
        gfa_goal_sf: gfa_goal,
        preliminary_area_budget,
        footprint_seed_sf: plate_seed,
        podium_footprint_sf,
        upper_footprint_sf,
        story_count,
        construction_case: classify_construction_case(input.building_construction_type),
        shape_case: classify_shape_case(shape),
        shape_diagnostics,
        concept_blocks: Vec::new(),
        family_block_bindings: Vec::new(),
        site_plan_bundle: MassingSitePlanBundle {
            bundle_id: "site_plan_uninitialized".to_string(),
            california_site_mode: input.site_planning.california_mode,
            overlay_binding_mode: input.site_planning.overlay.binding_mode,
            overlay_reference: input.site_planning.overlay.overlay_id.clone(),
            buildable_envelope: SiteBuildableEnvelope {
                envelope_id: "site_plan_uninitialized".to_string(),
                parcel_polygon: input.site_polygon.clone(),
                buildable_polygon: Vec::new(),
                frontage_edge_indices: Vec::new(),
                no_build_edge_indices: Vec::new(),
                fallback_mode: "uninitialized".to_string(),
                issue_codes: Vec::new(),
                confidence: 0.0,
                notes: Vec::new(),
            },
            frontage_candidates: Vec::new(),
            reservations: Vec::new(),
            anchor_points: Vec::new(),
            segments: Vec::new(),
            site_zones: Vec::new(),
            parking_topology: SiteParkingTopology {
                topology_id: "site_plan_uninitialized".to_string(),
                allocations: Vec::new(),
                lot_cells: Vec::new(),
                notes: Vec::new(),
            },
            maneuver_checks: Vec::new(),
            clearance_checks: Vec::new(),
            outdoor_topology_graph: OutdoorSiteTopologyGraph {
                graph_id: "site_plan_uninitialized".to_string(),
                nodes: Vec::new(),
                edges: Vec::new(),
                notes: Vec::new(),
            },
            concept_volumes: Vec::new(),
            score_breakdown: SitePlanScoreBreakdown {
                far_priority_score: 0.0,
                dwelling_priority_score: 0.0,
                site_feasibility_penalty: 0.0,
                access_penalty: 0.0,
                parking_penalty: 0.0,
                privacy_penalty: 0.0,
                clearance_penalty: 0.0,
                total_score: 0.0,
                notes: Vec::new(),
            },
            diagnostics: Vec::new(),
            notes: Vec::new(),
        },
    };
    massing.site_plan_bundle = build_site_plan_bundle(input, a, &massing);
    massing
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BboxSide {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
struct OrientedSiteFrame {
    origin: Point2,
    u_axis: Point2,
    v_axis: Point2,
    min_u: f64,
    max_u: f64,
    min_v: f64,
    max_v: f64,
    front_is_min: bool,
}

#[derive(Debug, Clone)]
struct SiteDemandEstimate {
    unit_counts: [u32; 4],
    dwelling_units: u32,
    avg_unit_area_sf: f64,
    loading_zone_count: u32,
    parking_required_stalls: u32,
    parking_provided_stalls: u32,
    accessible_stalls: u32,
    outdoor_open_space_target_sf: f64,
}

fn point_mean(poly: &[Point2]) -> Point2 {
    if poly.is_empty() {
        return Point2::new(0.0, 0.0);
    }
    let sum = poly
        .iter()
        .copied()
        .fold(Point2::new(0.0, 0.0), |acc, point| acc.add(point));
    sum.scale(1.0 / poly.len() as f64)
}

fn edge_points(poly: &[Point2], idx: usize) -> (Point2, Point2) {
    let a = poly[idx % poly.len()];
    let b = poly[(idx + 1) % poly.len()];
    (a, b)
}

fn edge_midpoint(a: Point2, b: Point2) -> Point2 {
    Point2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

fn project_to_frame(point: Point2, origin: Point2, u_axis: Point2, v_axis: Point2) -> (f64, f64) {
    let delta = point.sub(origin);
    (delta.dot(u_axis), delta.dot(v_axis))
}

fn point_from_frame(origin: Point2, u_axis: Point2, v_axis: Point2, u: f64, v: f64) -> Point2 {
    origin.add(u_axis.scale(u)).add(v_axis.scale(v))
}

fn oriented_rect_polygon(
    origin: Point2,
    u_axis: Point2,
    v_axis: Point2,
    min_u: f64,
    max_u: f64,
    min_v: f64,
    max_v: f64,
) -> Vec<Point2> {
    vec![
        point_from_frame(origin, u_axis, v_axis, min_u, min_v),
        point_from_frame(origin, u_axis, v_axis, max_u, min_v),
        point_from_frame(origin, u_axis, v_axis, max_u, max_v),
        point_from_frame(origin, u_axis, v_axis, min_u, max_v),
    ]
}

fn bbox_side_for_point(point: Point2, bbox: Rect2) -> BboxSide {
    let left = (point.x - bbox.min_x).abs();
    let right = (bbox.max_x - point.x).abs();
    let bottom = (point.y - bbox.min_y).abs();
    let top = (bbox.max_y - point.y).abs();
    let mut best = (bottom, BboxSide::Bottom);
    for candidate in [
        (top, BboxSide::Top),
        (left, BboxSide::Left),
        (right, BboxSide::Right),
    ] {
        if candidate.0 < best.0 {
            best = candidate;
        }
    }
    best.1
}

fn side_length(bbox: Rect2, side: BboxSide) -> f64 {
    match side {
        BboxSide::Top | BboxSide::Bottom => bbox.width(),
        BboxSide::Left | BboxSide::Right => bbox.height(),
    }
}

fn side_inward_depth(outer: Rect2, inner: Rect2, side: BboxSide) -> f64 {
    match side {
        BboxSide::Bottom => (inner.min_y - outer.min_y).max(0.0),
        BboxSide::Top => (outer.max_y - inner.max_y).max(0.0),
        BboxSide::Left => (inner.min_x - outer.min_x).max(0.0),
        BboxSide::Right => (outer.max_x - inner.max_x).max(0.0),
    }
}

fn fraction_on_side(point: Point2, bbox: Rect2, side: BboxSide) -> f64 {
    match side {
        BboxSide::Top | BboxSide::Bottom => {
            if bbox.width() <= EPS {
                0.5
            } else {
                clamp((point.x - bbox.min_x) / bbox.width(), 0.0, 1.0)
            }
        }
        BboxSide::Left | BboxSide::Right => {
            if bbox.height() <= EPS {
                0.5
            } else {
                clamp((point.y - bbox.min_y) / bbox.height(), 0.0, 1.0)
            }
        }
    }
}

fn band_rect_for_side(
    side: BboxSide,
    outer: Rect2,
    inner: Rect2,
    center_fraction: f64,
    span_ratio: f64,
    depth_override: Option<f64>,
) -> Option<Rect2> {
    let side_len = side_length(outer, side);
    let depth = depth_override.unwrap_or_else(|| side_inward_depth(outer, inner, side));
    if side_len <= EPS || depth <= EPS {
        return None;
    }
    let span = clamp(span_ratio, 0.12, 1.0) * side_len;
    let center = match side {
        BboxSide::Top | BboxSide::Bottom => {
            outer.min_x + clamp(center_fraction, 0.0, 1.0) * outer.width()
        }
        BboxSide::Left | BboxSide::Right => {
            outer.min_y + clamp(center_fraction, 0.0, 1.0) * outer.height()
        }
    };
    match side {
        BboxSide::Bottom => {
            let min_x = clamp(center - span * 0.5, outer.min_x, outer.max_x);
            let max_x = clamp(center + span * 0.5, outer.min_x, outer.max_x);
            Some(Rect2::new(
                min_x,
                outer.min_y,
                max_x.max(min_x + 1.0),
                (outer.min_y + depth).min(inner.min_y.max(outer.min_y + depth)),
            ))
        }
        BboxSide::Top => {
            let min_x = clamp(center - span * 0.5, outer.min_x, outer.max_x);
            let max_x = clamp(center + span * 0.5, outer.min_x, outer.max_x);
            Some(Rect2::new(
                min_x,
                (outer.max_y - depth).max(inner.max_y.min(outer.max_y - depth)),
                max_x.max(min_x + 1.0),
                outer.max_y,
            ))
        }
        BboxSide::Left => {
            let min_y = clamp(center - span * 0.5, outer.min_y, outer.max_y);
            let max_y = clamp(center + span * 0.5, outer.min_y, outer.max_y);
            Some(Rect2::new(
                outer.min_x,
                min_y,
                (outer.min_x + depth).min(inner.min_x.max(outer.min_x + depth)),
                max_y.max(min_y + 1.0),
            ))
        }
        BboxSide::Right => {
            let min_y = clamp(center - span * 0.5, outer.min_y, outer.max_y);
            let max_y = clamp(center + span * 0.5, outer.min_y, outer.max_y);
            Some(Rect2::new(
                (outer.max_x - depth).max(inner.max_x.min(outer.max_x - depth)),
                min_y,
                outer.max_x,
                max_y.max(min_y + 1.0),
            ))
        }
    }
}

fn zone_name(kind: SitePlanProgramKind) -> &'static str {
    match kind {
        SitePlanProgramKind::ArrivalForecourt => "Arrival Forecourt",
        SitePlanProgramKind::LoadingZone => "Loading Zone",
        SitePlanProgramKind::ServiceYard => "Service Yard",
        SitePlanProgramKind::FireAccessBand => "Fire Access Band",
        SitePlanProgramKind::PublicWalk => "Public Walk",
        SitePlanProgramKind::AccessibleWalk => "Accessible Walk",
        SitePlanProgramKind::ParkingSurface => "Surface Parking",
        SitePlanProgramKind::DriveAisle => "Drive Aisle",
        SitePlanProgramKind::ParkingWalk => "Parking Walk",
        SitePlanProgramKind::LandscapeZone => "Landscape Zone",
        SitePlanProgramKind::OpenSpaceZone => "Open Space Zone",
        SitePlanProgramKind::PrivacyBuffer => "Privacy Buffer",
        SitePlanProgramKind::ResidualDevelopable => "Residual Developable",
        SitePlanProgramKind::BuildingFootprint => "Building Footprint",
        SitePlanProgramKind::PodiumEnvelope => "Podium Envelope",
        SitePlanProgramKind::BelowGradeParkingEnvelope => "Below Grade Parking Envelope",
    }
}

fn frontage_role_score(frontage: &SiteFrontageCandidate, role: SitePlanFrontageRole) -> f64 {
    match role {
        SitePlanFrontageRole::PublicEntry => frontage.public_score,
        SitePlanFrontageRole::ServiceLoading => frontage.service_score,
        SitePlanFrontageRole::FireAccess => frontage.fire_score,
        SitePlanFrontageRole::ParkingAccess => frontage.parking_score,
        SitePlanFrontageRole::PrivacySensitive => frontage.privacy_score,
    }
}

fn choose_primary_frontage_index(input: &NormalizedInput) -> usize {
    if let Some(idx) = input.site_planning.frontage.frontage_edge_indices.first() {
        return *idx % input.site_polygon.len().max(1);
    }
    if let Some(idx) = input.site_planning.frontage.entry_edge_indices.first() {
        return *idx % input.site_polygon.len().max(1);
    }
    let mut best_idx = 0usize;
    let mut best_len = -1.0;
    for idx in 0..input.site_polygon.len() {
        let (a, b) = edge_points(&input.site_polygon, idx);
        let len = a.distance_to(b);
        if len > best_len {
            best_len = len;
            best_idx = idx;
        }
    }
    best_idx
}

fn build_site_frame(input: &NormalizedInput, primary_frontage_idx: usize) -> OrientedSiteFrame {
    let (start, end) = edge_points(&input.site_polygon, primary_frontage_idx);
    let origin = start;
    let u_axis = end.sub(start).normalized();
    let mut v_axis = u_axis.perp();
    let centroid = point_mean(&input.site_polygon);
    let front_mid = edge_midpoint(start, end);
    if centroid.sub(front_mid).dot(v_axis) < 0.0 {
        v_axis = v_axis.scale(-1.0);
    }
    let mut min_u = f64::INFINITY;
    let mut max_u = f64::NEG_INFINITY;
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for point in &input.site_polygon {
        let (u, v) = project_to_frame(*point, origin, u_axis, v_axis);
        min_u = min_u.min(u);
        max_u = max_u.max(u);
        min_v = min_v.min(v);
        max_v = max_v.max(v);
    }
    let (_, front_v) = project_to_frame(front_mid, origin, u_axis, v_axis);
    let front_is_min = (front_v - min_v).abs() <= (front_v - max_v).abs();
    OrientedSiteFrame {
        origin,
        u_axis,
        v_axis,
        min_u,
        max_u,
        min_v,
        max_v,
        front_is_min,
    }
}

fn edge_setback(input: &NormalizedInput, edge_idx: usize, fallback: f64) -> f64 {
    input
        .site_planning
        .buildable
        .edge_setbacks_ft
        .get(edge_idx)
        .copied()
        .unwrap_or(fallback)
        .max(0.0)
}

fn build_buildable_envelope(
    input: &NormalizedInput,
    primary_frontage_idx: usize,
) -> SiteBuildableEnvelope {
    let frame = build_site_frame(input, primary_frontage_idx);
    let front_indices = if input
        .site_planning
        .frontage
        .frontage_edge_indices
        .is_empty()
    {
        vec![primary_frontage_idx]
    } else {
        input.site_planning.frontage.frontage_edge_indices.clone()
    };
    let mut rear_idx = primary_frontage_idx;
    let mut rear_score = f64::NEG_INFINITY;
    for idx in 0..input.site_polygon.len() {
        let midpoint = edge_midpoint(
            edge_points(&input.site_polygon, idx).0,
            edge_points(&input.site_polygon, idx).1,
        );
        let (_, v) = project_to_frame(midpoint, frame.origin, frame.u_axis, frame.v_axis);
        let score = if frame.front_is_min { v } else { -v };
        if score > rear_score {
            rear_score = score;
            rear_idx = idx;
        }
    }
    let side_indices = (0..input.site_polygon.len())
        .filter(|idx| !front_indices.contains(idx) && *idx != rear_idx)
        .collect::<Vec<_>>();

    let mut front_setback = if front_indices.is_empty() {
        input.site_planning.buildable.default_front_setback_ft
    } else {
        front_indices
            .iter()
            .map(|idx| {
                edge_setback(
                    input,
                    *idx,
                    input.site_planning.buildable.default_front_setback_ft,
                )
            })
            .sum::<f64>()
            / front_indices.len() as f64
    };
    let mut rear_setback = edge_setback(
        input,
        rear_idx,
        input.site_planning.buildable.default_rear_setback_ft,
    );
    let mut side_setback = if side_indices.is_empty() {
        input.site_planning.buildable.default_side_setback_ft
    } else {
        side_indices
            .iter()
            .map(|idx| {
                edge_setback(
                    input,
                    *idx,
                    input.site_planning.buildable.default_side_setback_ft,
                )
            })
            .sum::<f64>()
            / side_indices.len() as f64
    };

    if input
        .site_planning
        .buildable
        .no_build_edge_indices
        .iter()
        .any(|idx| front_indices.contains(idx))
    {
        front_setback += input
            .site_planning
            .buildable
            .default_front_setback_ft
            .max(input.site_planning.buildable.setback_snap_ft);
    }
    if input
        .site_planning
        .buildable
        .no_build_edge_indices
        .contains(&rear_idx)
    {
        rear_setback += input
            .site_planning
            .buildable
            .default_rear_setback_ft
            .max(input.site_planning.buildable.setback_snap_ft);
    }
    if input
        .site_planning
        .buildable
        .no_build_edge_indices
        .iter()
        .any(|idx| side_indices.contains(idx))
    {
        side_setback += input
            .site_planning
            .buildable
            .default_side_setback_ft
            .max(input.site_planning.buildable.setback_snap_ft);
    }

    let min_u = frame.min_u + side_setback;
    let max_u = frame.max_u - side_setback;
    let (min_v, max_v) = if frame.front_is_min {
        (frame.min_v + front_setback, frame.max_v - rear_setback)
    } else {
        (frame.min_v + rear_setback, frame.max_v - front_setback)
    };

    let mut issue_codes = Vec::<String>::new();
    let mut fallback_mode = "oriented_inset".to_string();
    let buildable_polygon = if max_u <= min_u + EPS || max_v <= min_v + EPS {
        fallback_mode = "bbox_inset_fallback".to_string();
        issue_codes.push("buildable_bounds_collapsed".to_string());
        let bbox = bounding_rect(&input.site_polygon);
        bbox.inset(
            ((front_setback + rear_setback + side_setback) / 3.0).max(0.0),
            ((front_setback + rear_setback + side_setback) / 3.0).max(0.0),
        )
        .to_polygon()
    } else {
        oriented_rect_polygon(
            frame.origin,
            frame.u_axis,
            frame.v_axis,
            min_u,
            max_u,
            min_v,
            max_v,
        )
    };

    let buildable_area = polygon_area_sf(&buildable_polygon);
    if buildable_area < input.site_planning.buildable.min_buildable_area_sf {
        issue_codes.push("buildable_area_below_minimum".to_string());
    }
    let buildable_bbox = bounding_rect(&buildable_polygon);
    if buildable_bbox.width().min(buildable_bbox.height())
        < input.site_planning.buildable.default_side_setback_ft * 1.5
    {
        issue_codes.push("narrow_neck_risk".to_string());
    }
    if front_setback > (frame.max_v - frame.min_v).abs() * 0.35 {
        issue_codes.push("frontage_loss_risk".to_string());
    }

    SiteBuildableEnvelope {
        envelope_id: "parcel_buildable_envelope".to_string(),
        parcel_polygon: input.site_polygon.clone(),
        buildable_polygon,
        frontage_edge_indices: front_indices,
        no_build_edge_indices: input.site_planning.buildable.no_build_edge_indices.clone(),
        fallback_mode,
        issue_codes: issue_codes.clone(),
        confidence: clamp(0.92 - issue_codes.len() as f64 * 0.12, 0.25, 0.95),
        notes: vec![
            format!("front_setback_ft={:.2}", front_setback),
            format!("side_setback_ft={:.2}", side_setback),
            format!("rear_setback_ft={:.2}", rear_setback),
        ],
    }
}

fn build_frontage_candidates(
    input: &NormalizedInput,
    buildable: &SiteBuildableEnvelope,
) -> Vec<SiteFrontageCandidate> {
    let mut out = Vec::<SiteFrontageCandidate>::new();
    if input.site_polygon.len() < 2 {
        return out;
    }
    let max_len = (0..input.site_polygon.len())
        .map(|idx| {
            let (a, b) = edge_points(&input.site_polygon, idx);
            a.distance_to(b)
        })
        .fold(1.0, f64::max);

    for edge_index in 0..input.site_polygon.len() {
        let (start, end) = edge_points(&input.site_polygon, edge_index);
        let length_ft = start.distance_to(end);
        let len_ratio = clamp(length_ft / max_len.max(1.0), 0.0, 1.0);
        let no_build = buildable.no_build_edge_indices.contains(&edge_index);
        let public_hint = input
            .site_planning
            .frontage
            .frontage_edge_indices
            .contains(&edge_index);
        let entry_hint = input
            .site_planning
            .frontage
            .entry_edge_indices
            .contains(&edge_index);
        let service_hint = input
            .site_planning
            .frontage
            .service_access_edge_indices
            .contains(&edge_index);
        let fire_hint = input
            .site_planning
            .frontage
            .fire_access_edge_indices
            .contains(&edge_index);
        let privacy_hint = input
            .site_planning
            .frontage
            .privacy_edge_indices
            .contains(&edge_index);

        let public_score = clamp(
            0.35 + 0.55 * len_ratio
                + if public_hint { 0.45 } else { 0.0 }
                + if entry_hint { 0.20 } else { 0.0 }
                - if no_build { 0.30 } else { 0.0 },
            0.0,
            1.5,
        );
        let service_score = clamp(
            0.20 + 0.45 * len_ratio
                + if service_hint { 0.50 } else { 0.0 }
                + if input.site_planning.frontage.allow_service_on_public_front && public_hint {
                    0.10
                } else {
                    0.0
                }
                - if privacy_hint { 0.20 } else { 0.0 },
            0.0,
            1.5,
        );
        let fire_score = clamp(
            0.25 + 0.45 * len_ratio
                + if fire_hint { 0.45 } else { 0.0 }
                + if public_hint { 0.08 } else { 0.0 },
            0.0,
            1.5,
        );
        let parking_score = clamp(
            0.25 + 0.40 * len_ratio
                + if service_hint { 0.20 } else { 0.0 }
                + if public_hint { 0.10 } else { 0.0 },
            0.0,
            1.5,
        );
        let privacy_score = clamp(
            0.18 + 0.35 * len_ratio
                + if privacy_hint { 0.60 } else { 0.0 }
                + if no_build { 0.25 } else { 0.0 }
                - if public_hint { 0.15 } else { 0.0 },
            0.0,
            1.5,
        );

        out.push(SiteFrontageCandidate {
            frontage_id: format!("frontage_E{:02}", edge_index),
            edge_index,
            start,
            end,
            length_ft,
            public_score,
            service_score,
            fire_score,
            parking_score,
            privacy_score,
            active_roles: Vec::new(),
            accepted: false,
            notes: Vec::new(),
        });
    }

    let public_max = out.iter().map(|x| x.public_score).fold(0.0, f64::max);
    let service_max = out.iter().map(|x| x.service_score).fold(0.0, f64::max);
    let fire_max = out.iter().map(|x| x.fire_score).fold(0.0, f64::max);
    let parking_max = out.iter().map(|x| x.parking_score).fold(0.0, f64::max);
    let privacy_max = out.iter().map(|x| x.privacy_score).fold(0.0, f64::max);
    let multi_front_delta = if input.site_planning.frontage.prioritize_multiple_fronts {
        0.15
    } else {
        0.04
    };

    for frontage in &mut out {
        let explicit_public = input
            .site_planning
            .frontage
            .frontage_edge_indices
            .contains(&frontage.edge_index)
            || input
                .site_planning
                .frontage
                .entry_edge_indices
                .contains(&frontage.edge_index);
        let explicit_service = input
            .site_planning
            .frontage
            .service_access_edge_indices
            .contains(&frontage.edge_index);
        let explicit_fire = input
            .site_planning
            .frontage
            .fire_access_edge_indices
            .contains(&frontage.edge_index);
        let explicit_privacy = input
            .site_planning
            .frontage
            .privacy_edge_indices
            .contains(&frontage.edge_index)
            || buildable
                .no_build_edge_indices
                .contains(&frontage.edge_index);

        if frontage.public_score >= public_max - multi_front_delta || explicit_public {
            frontage
                .active_roles
                .push(SitePlanFrontageRole::PublicEntry);
        }
        if frontage.service_score >= service_max - 0.10 || explicit_service {
            if input.site_planning.frontage.allow_service_on_public_front
                || !frontage
                    .active_roles
                    .contains(&SitePlanFrontageRole::PublicEntry)
                || explicit_service
            {
                frontage
                    .active_roles
                    .push(SitePlanFrontageRole::ServiceLoading);
            } else {
                frontage
                    .notes
                    .push("service_role_downgraded_due_to_public_priority".to_string());
            }
        }
        if frontage.fire_score >= fire_max - 0.12 || explicit_fire {
            frontage.active_roles.push(SitePlanFrontageRole::FireAccess);
        }
        if input.constraints.parking_mode != ParkingMode::None
            && frontage.parking_score >= parking_max - 0.12
        {
            frontage
                .active_roles
                .push(SitePlanFrontageRole::ParkingAccess);
        }
        if frontage.privacy_score >= privacy_max - 0.10 || explicit_privacy {
            frontage
                .active_roles
                .push(SitePlanFrontageRole::PrivacySensitive);
        }
        frontage.accepted = !frontage.active_roles.is_empty();
        if !frontage.accepted {
            frontage
                .notes
                .push("frontage_rejected_for_all_roles".to_string());
        }
    }

    out
}

fn select_frontage_ids(
    frontages: &[SiteFrontageCandidate],
    role: SitePlanFrontageRole,
) -> Vec<String> {
    let mut selected = frontages
        .iter()
        .filter(|frontage| frontage.active_roles.contains(&role))
        .cloned()
        .collect::<Vec<_>>();
    selected.sort_by(|a, b| {
        frontage_role_score(b, role)
            .partial_cmp(&frontage_role_score(a, role))
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.edge_index.cmp(&b.edge_index))
    });
    selected
        .into_iter()
        .map(|frontage| frontage.frontage_id)
        .collect()
}

fn estimate_site_demands(
    input: &NormalizedInput,
    massing: &MassingState,
    a: &AssumptionPack,
) -> SiteDemandEstimate {
    let avg_unit_area_sf = weighted_seed_target_area(input, a).max(1.0);
    let net_residential_area_sf =
        (massing.gfa_goal_sf - input.targets.retail_area_sf).max(avg_unit_area_sf);
    let mut dwelling_units = (net_residential_area_sf * 0.78 / avg_unit_area_sf)
        .floor()
        .max(1.0) as u32;
    if let Some(cap) = input.targets.dwelling_units_cap {
        dwelling_units = dwelling_units.min(cap.max(1));
    }

    let mix = [
        input.unit_mix_seed.studio,
        input.unit_mix_seed.one_bedroom,
        input.unit_mix_seed.two_bedroom,
        input.unit_mix_seed.three_bedroom,
    ];
    let mut counts = [0u32; 4];
    let mut floor_total = 0u32;
    let mut fractions = Vec::<(usize, f64)>::new();
    for (idx, ratio) in mix.iter().copied().enumerate() {
        let raw = ratio * dwelling_units as f64;
        let base = raw.floor() as u32;
        counts[idx] = base;
        floor_total += base;
        fractions.push((idx, raw - base as f64));
    }
    fractions.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    for (idx, _) in fractions {
        if floor_total >= dwelling_units {
            break;
        }
        counts[idx] += 1;
        floor_total += 1;
    }

    let guest_stalls = input
        .site_planning
        .parking
        .guest_stalls_per_du
        .map(|ratio| (ratio.max(0.0) * dwelling_units as f64).ceil() as u32)
        .unwrap_or(0);
    let mut parking_required_stalls =
        parking_stall_demand(counts, input.targets.retail_area_sf, input, a)
            .saturating_add(guest_stalls);
    parking_required_stalls = ((parking_required_stalls as f64)
        * (1.0 - clamp(input.site_planning.parking.tdm_reduction_ratio, 0.0, 0.75)))
    .ceil()
    .max(0.0) as u32;
    let parking_provided_stalls = input
        .site_planning
        .parking
        .provided_stalls_cap
        .map(|cap| parking_required_stalls.min(cap))
        .unwrap_or(parking_required_stalls);
    let accessible_ratio = input
        .site_planning
        .parking
        .accessible_stall_ratio
        .unwrap_or(0.04);
    let accessible_stalls = if parking_provided_stalls == 0 {
        0
    } else {
        ((parking_provided_stalls as f64) * accessible_ratio)
            .ceil()
            .max(1.0) as u32
    };

    SiteDemandEstimate {
        unit_counts: counts,
        dwelling_units,
        avg_unit_area_sf,
        loading_zone_count: loading_zone_count(dwelling_units),
        parking_required_stalls,
        parking_provided_stalls,
        accessible_stalls,
        outdoor_open_space_target_sf: input
            .amenities
            .outdoor_target_sf
            .unwrap_or(dwelling_units as f64 * a.amenity.outdoor_min_sf_per_du),
    }
}

fn build_parking_allocations(
    input: &NormalizedInput,
    demand: &SiteDemandEstimate,
    a: &AssumptionPack,
) -> Vec<SiteParkingAllocation> {
    let provided = demand.parking_provided_stalls;
    let mode_allocations = match input.constraints.parking_mode {
        ParkingMode::None => Vec::new(),
        ParkingMode::Surface => vec![(ParkingMode::Surface, 1.0)],
        ParkingMode::Podium => vec![(ParkingMode::Podium, 1.0)],
        ParkingMode::Structured => vec![(ParkingMode::Structured, 1.0)],
        ParkingMode::Underground => vec![(ParkingMode::Underground, 1.0)],
        ParkingMode::Mixed => vec![
            (ParkingMode::Surface, 0.35),
            (ParkingMode::Podium, 0.30),
            (ParkingMode::Underground, 0.35),
        ],
        ParkingMode::Auto => {
            if input.levels.count <= 4 {
                vec![
                    (ParkingMode::Surface, 0.55),
                    (ParkingMode::Structured, 0.45),
                ]
            } else {
                vec![
                    (ParkingMode::Surface, 0.25),
                    (ParkingMode::Podium, 0.35),
                    (ParkingMode::Underground, 0.40),
                ]
            }
        }
    };

    let mut out = Vec::<SiteParkingAllocation>::new();
    let mut assigned = 0u32;
    for (idx, (mode, share)) in mode_allocations.iter().enumerate() {
        let mut reserved_stalls = if idx + 1 == mode_allocations.len() {
            provided.saturating_sub(assigned)
        } else {
            ((provided as f64) * share).round() as u32
        };
        reserved_stalls = reserved_stalls.min(provided.saturating_sub(assigned));
        assigned += reserved_stalls;
        let accessible_stalls = if provided == 0 {
            0
        } else {
            ((demand.accessible_stalls as f64) * (reserved_stalls as f64 / provided as f64)).round()
                as u32
        };
        out.push(SiteParkingAllocation {
            allocation_id: format!("parking_alloc_{:?}_{:02}", mode, idx),
            parking_mode: *mode,
            reserved_stalls,
            accessible_stalls,
            reserved_area_sf: reserved_stalls as f64 * parking_gross_sf_per_stall(*mode, a),
            active: reserved_stalls > 0,
            notes: vec![format!("share={:.2}", share)],
        });
    }
    out
}

fn build_reservations(
    input: &NormalizedInput,
    massing: &MassingState,
    buildable: &SiteBuildableEnvelope,
    frontages: &[SiteFrontageCandidate],
    demand: &SiteDemandEstimate,
    parking_allocations: &[SiteParkingAllocation],
    a: &AssumptionPack,
) -> Vec<SiteReservation> {
    let buildable_area = polygon_area_sf(&buildable.buildable_polygon);
    let public_fronts = select_frontage_ids(frontages, SitePlanFrontageRole::PublicEntry);
    let service_fronts = select_frontage_ids(frontages, SitePlanFrontageRole::ServiceLoading);
    let fire_fronts = select_frontage_ids(frontages, SitePlanFrontageRole::FireAccess);
    let privacy_fronts = select_frontage_ids(frontages, SitePlanFrontageRole::PrivacySensitive);
    let surface_area = parking_allocations
        .iter()
        .filter(|alloc| alloc.parking_mode == ParkingMode::Surface)
        .map(|alloc| alloc.reserved_area_sf)
        .sum::<f64>();
    let arrival_target = (massing.site_area_sf * 0.015).max(650.0);
    let loading_target = demand.loading_zone_count.max(1) as f64
        * input
            .site_planning
            .loading
            .loading_zone_area_sf
            .unwrap_or(a.support.loading_zone_default_sf);
    let fire_target = if input.site_planning.fire_access.required {
        bounding_rect(&buildable.buildable_polygon)
            .width()
            .max(40.0)
            * input
                .site_planning
                .clearance
                .fire_lane_width_ft
                .unwrap_or(20.0)
            * 0.55
    } else {
        0.0
    };
    let public_walk_target = input
        .site_planning
        .clearance
        .public_walk_width_ft
        .unwrap_or(10.0)
        * bounding_rect(&buildable.buildable_polygon)
            .height()
            .max(30.0);
    let accessible_walk_target = input
        .site_planning
        .clearance
        .accessible_walk_width_ft
        .unwrap_or(8.0)
        * bounding_rect(&buildable.buildable_polygon)
            .height()
            .max(30.0);
    let landscape_target = demand
        .outdoor_open_space_target_sf
        .max(massing.site_area_sf * 0.08);
    let privacy_target = privacy_fronts.len().max(1) as f64
        * input
            .site_planning
            .privacy
            .screening_depth_ft
            .unwrap_or(10.0)
        * 24.0;
    let residual_target = buildable_area.max(0.0);

    vec![
        SiteReservation {
            reservation_id: "reservation_arrival".to_string(),
            program_kind: SitePlanProgramKind::ArrivalForecourt,
            priority_rank: 0,
            target_area_sf: arrival_target,
            reserved_area_sf: arrival_target,
            perimeter_claim_ft: public_fronts.len().max(1) as f64 * 24.0,
            linked_frontage_ids: public_fronts,
            shortfall_area_sf: 0.0,
            notes: vec!["public_arrival_priority".to_string()],
        },
        SiteReservation {
            reservation_id: "reservation_loading".to_string(),
            program_kind: SitePlanProgramKind::LoadingZone,
            priority_rank: 1,
            target_area_sf: loading_target,
            reserved_area_sf: loading_target,
            perimeter_claim_ft: service_fronts.len().max(1) as f64 * 28.0,
            linked_frontage_ids: service_fronts.clone(),
            shortfall_area_sf: 0.0,
            notes: vec![format!("loading_zone_count={}", demand.loading_zone_count)],
        },
        SiteReservation {
            reservation_id: "reservation_fire".to_string(),
            program_kind: SitePlanProgramKind::FireAccessBand,
            priority_rank: 2,
            target_area_sf: fire_target,
            reserved_area_sf: fire_target,
            perimeter_claim_ft: fire_fronts.len().max(1) as f64 * 40.0,
            linked_frontage_ids: fire_fronts,
            shortfall_area_sf: 0.0,
            notes: vec![],
        },
        SiteReservation {
            reservation_id: "reservation_public_walk".to_string(),
            program_kind: SitePlanProgramKind::PublicWalk,
            priority_rank: 3,
            target_area_sf: public_walk_target,
            reserved_area_sf: public_walk_target,
            perimeter_claim_ft: 0.0,
            linked_frontage_ids: Vec::new(),
            shortfall_area_sf: 0.0,
            notes: vec![],
        },
        SiteReservation {
            reservation_id: "reservation_accessible_walk".to_string(),
            program_kind: SitePlanProgramKind::AccessibleWalk,
            priority_rank: 4,
            target_area_sf: accessible_walk_target,
            reserved_area_sf: accessible_walk_target,
            perimeter_claim_ft: 0.0,
            linked_frontage_ids: Vec::new(),
            shortfall_area_sf: 0.0,
            notes: vec![],
        },
        SiteReservation {
            reservation_id: "reservation_surface_parking".to_string(),
            program_kind: SitePlanProgramKind::ParkingSurface,
            priority_rank: 5,
            target_area_sf: surface_area,
            reserved_area_sf: surface_area,
            perimeter_claim_ft: 0.0,
            linked_frontage_ids: select_frontage_ids(
                frontages,
                SitePlanFrontageRole::ParkingAccess,
            ),
            shortfall_area_sf: 0.0,
            notes: vec![format!(
                "parking_required_stalls={}",
                demand.parking_required_stalls
            )],
        },
        SiteReservation {
            reservation_id: "reservation_landscape".to_string(),
            program_kind: SitePlanProgramKind::LandscapeZone,
            priority_rank: 6,
            target_area_sf: landscape_target,
            reserved_area_sf: landscape_target,
            perimeter_claim_ft: 0.0,
            linked_frontage_ids: privacy_fronts.clone(),
            shortfall_area_sf: 0.0,
            notes: vec![],
        },
        SiteReservation {
            reservation_id: "reservation_privacy".to_string(),
            program_kind: SitePlanProgramKind::PrivacyBuffer,
            priority_rank: 7,
            target_area_sf: privacy_target,
            reserved_area_sf: privacy_target,
            perimeter_claim_ft: privacy_fronts.len().max(1) as f64 * 18.0,
            linked_frontage_ids: privacy_fronts,
            shortfall_area_sf: 0.0,
            notes: vec![],
        },
        SiteReservation {
            reservation_id: "reservation_residual".to_string(),
            program_kind: SitePlanProgramKind::ResidualDevelopable,
            priority_rank: 8,
            target_area_sf: residual_target,
            reserved_area_sf: residual_target,
            perimeter_claim_ft: 0.0,
            linked_frontage_ids: Vec::new(),
            shortfall_area_sf: 0.0,
            notes: vec!["buildable_residual_budget".to_string()],
        },
    ]
}

fn make_anchor(
    anchor_id: &str,
    anchor_kind: SitePlanAnchorKind,
    point: Point2,
    linked_frontage_id: Option<String>,
    linked_reservation_id: Option<String>,
    required: bool,
    notes: Vec<String>,
) -> SiteAnchorPoint {
    SiteAnchorPoint {
        anchor_id: anchor_id.to_string(),
        anchor_kind,
        point,
        linked_frontage_id,
        linked_reservation_id,
        required,
        notes,
    }
}

fn make_segment(
    segment_id: &str,
    segment_kind: SitePlanSegmentKind,
    from_anchor_id: &str,
    to_anchor_id: &str,
    geometry: Vec<Point2>,
    width_ft: f64,
    required: bool,
    linked_zone_ids: Vec<String>,
    notes: Vec<String>,
) -> SiteSegment {
    SiteSegment {
        segment_id: segment_id.to_string(),
        segment_kind,
        from_anchor_id: from_anchor_id.to_string(),
        to_anchor_id: to_anchor_id.to_string(),
        geometry,
        width_ft,
        required,
        linked_zone_ids,
        notes,
    }
}

fn make_zone(
    zone_id: &str,
    zone_kind: SitePlanProgramKind,
    rect: Rect2,
    linked_frontage_ids: Vec<String>,
    linked_anchor_ids: Vec<String>,
    linked_segment_ids: Vec<String>,
    residual: bool,
    notes: Vec<String>,
) -> SiteZone {
    SiteZone {
        zone_id: zone_id.to_string(),
        zone_kind,
        polygon: rect.to_polygon(),
        area_sf: rect.area(),
        linked_frontage_ids,
        linked_anchor_ids,
        linked_segment_ids,
        residual,
        notes,
    }
}

fn map_zone_kind_to_node_kind(kind: SitePlanProgramKind) -> Option<OutdoorSiteNodeKind> {
    match kind {
        SitePlanProgramKind::ArrivalForecourt => Some(OutdoorSiteNodeKind::ArrivalEntry),
        SitePlanProgramKind::LoadingZone => Some(OutdoorSiteNodeKind::LoadingZone),
        SitePlanProgramKind::ServiceYard => Some(OutdoorSiteNodeKind::ServiceAccess),
        SitePlanProgramKind::FireAccessBand => Some(OutdoorSiteNodeKind::FireAccess),
        SitePlanProgramKind::PublicWalk => Some(OutdoorSiteNodeKind::PedestrianRoute),
        SitePlanProgramKind::AccessibleWalk => Some(OutdoorSiteNodeKind::AccessibleRoute),
        SitePlanProgramKind::ParkingSurface
        | SitePlanProgramKind::DriveAisle
        | SitePlanProgramKind::ParkingWalk => Some(OutdoorSiteNodeKind::ParkingWalk),
        SitePlanProgramKind::LandscapeZone => Some(OutdoorSiteNodeKind::LandscapeZone),
        SitePlanProgramKind::OpenSpaceZone => Some(OutdoorSiteNodeKind::OpenSpaceZone),
        SitePlanProgramKind::PrivacyBuffer => Some(OutdoorSiteNodeKind::PrivacyBuffer),
        SitePlanProgramKind::ResidualDevelopable => Some(OutdoorSiteNodeKind::OpenSpaceZone),
        SitePlanProgramKind::BuildingFootprint
        | SitePlanProgramKind::PodiumEnvelope
        | SitePlanProgramKind::BelowGradeParkingEnvelope => None,
    }
}

fn map_segment_kind_to_edge_kind(kind: SitePlanSegmentKind) -> OutdoorSiteEdgeKind {
    match kind {
        SitePlanSegmentKind::PublicWalk => OutdoorSiteEdgeKind::PedestrianFlow,
        SitePlanSegmentKind::AccessibleWalk => OutdoorSiteEdgeKind::AccessibleFlow,
        SitePlanSegmentKind::ServiceFlow => OutdoorSiteEdgeKind::ServiceFlow,
        SitePlanSegmentKind::FireFlow => OutdoorSiteEdgeKind::FireFlow,
        SitePlanSegmentKind::ParkingWalk | SitePlanSegmentKind::DriveAisle => {
            OutdoorSiteEdgeKind::ParkingWalkFlow
        }
        SitePlanSegmentKind::SeparationBuffer => OutdoorSiteEdgeKind::SeparationBuffer,
        SitePlanSegmentKind::ConflictCrossing => OutdoorSiteEdgeKind::ConflictCrossing,
    }
}

fn build_site_plan_bundle(
    input: &NormalizedInput,
    a: &AssumptionPack,
    massing: &MassingState,
) -> MassingSitePlanBundle {
    let primary_frontage_idx = choose_primary_frontage_index(input);
    let buildable_envelope = build_buildable_envelope(input, primary_frontage_idx);
    let frontages = build_frontage_candidates(input, &buildable_envelope);
    let demand = estimate_site_demands(input, massing, a);
    let parking_allocations = build_parking_allocations(input, &demand, a);
    let reservations = build_reservations(
        input,
        massing,
        &buildable_envelope,
        &frontages,
        &demand,
        &parking_allocations,
        a,
    );

    let site_bbox = bounding_rect(&input.site_polygon);
    let buildable_bbox = bounding_rect(&buildable_envelope.buildable_polygon);
    let footprint_rect = centered_rect_in_bbox(
        massing
            .upper_footprint_sf
            .max(400.0)
            .min(buildable_bbox.area().max(400.0)),
        buildable_bbox,
        if buildable_bbox.height() <= EPS {
            1.0
        } else {
            buildable_bbox.width().max(1.0) / buildable_bbox.height().max(1.0)
        },
    );

    let public_front_id = select_frontage_ids(&frontages, SitePlanFrontageRole::PublicEntry)
        .into_iter()
        .next();
    let service_front_id = select_frontage_ids(&frontages, SitePlanFrontageRole::ServiceLoading)
        .into_iter()
        .next()
        .or_else(|| public_front_id.clone());
    let fire_front_id = select_frontage_ids(&frontages, SitePlanFrontageRole::FireAccess)
        .into_iter()
        .next()
        .or_else(|| service_front_id.clone())
        .or_else(|| public_front_id.clone());
    let parking_front_id = select_frontage_ids(&frontages, SitePlanFrontageRole::ParkingAccess)
        .into_iter()
        .next()
        .or_else(|| service_front_id.clone())
        .or_else(|| public_front_id.clone());
    let privacy_front_id = select_frontage_ids(&frontages, SitePlanFrontageRole::PrivacySensitive)
        .into_iter()
        .next();

    let frontage_lookup = frontages
        .iter()
        .map(|frontage| (frontage.frontage_id.clone(), frontage.clone()))
        .collect::<BTreeMap<_, _>>();
    let public_side = public_front_id
        .as_ref()
        .and_then(|id| frontage_lookup.get(id))
        .map(|frontage| bbox_side_for_point(edge_midpoint(frontage.start, frontage.end), site_bbox))
        .unwrap_or(BboxSide::Bottom);
    let service_side = service_front_id
        .as_ref()
        .and_then(|id| frontage_lookup.get(id))
        .map(|frontage| bbox_side_for_point(edge_midpoint(frontage.start, frontage.end), site_bbox))
        .unwrap_or(BboxSide::Right);
    let fire_side = fire_front_id
        .as_ref()
        .and_then(|id| frontage_lookup.get(id))
        .map(|frontage| bbox_side_for_point(edge_midpoint(frontage.start, frontage.end), site_bbox))
        .unwrap_or(service_side);
    let parking_side = parking_front_id
        .as_ref()
        .and_then(|id| frontage_lookup.get(id))
        .map(|frontage| bbox_side_for_point(edge_midpoint(frontage.start, frontage.end), site_bbox))
        .unwrap_or(service_side);
    let privacy_side = privacy_front_id
        .as_ref()
        .and_then(|id| frontage_lookup.get(id))
        .map(|frontage| bbox_side_for_point(edge_midpoint(frontage.start, frontage.end), site_bbox))
        .unwrap_or(BboxSide::Left);

    let public_fraction = public_front_id
        .as_ref()
        .and_then(|id| frontage_lookup.get(id))
        .map(|frontage| {
            fraction_on_side(
                edge_midpoint(frontage.start, frontage.end),
                site_bbox,
                public_side,
            )
        })
        .unwrap_or(0.5);
    let service_fraction = service_front_id
        .as_ref()
        .and_then(|id| frontage_lookup.get(id))
        .map(|frontage| {
            fraction_on_side(
                edge_midpoint(frontage.start, frontage.end),
                site_bbox,
                service_side,
            )
        })
        .unwrap_or(0.8);
    let fire_fraction = fire_front_id
        .as_ref()
        .and_then(|id| frontage_lookup.get(id))
        .map(|frontage| {
            fraction_on_side(
                edge_midpoint(frontage.start, frontage.end),
                site_bbox,
                fire_side,
            )
        })
        .unwrap_or(0.65);
    let parking_fraction = parking_front_id
        .as_ref()
        .and_then(|id| frontage_lookup.get(id))
        .map(|frontage| {
            fraction_on_side(
                edge_midpoint(frontage.start, frontage.end),
                site_bbox,
                parking_side,
            )
        })
        .unwrap_or(0.75);
    let privacy_fraction = privacy_front_id
        .as_ref()
        .and_then(|id| frontage_lookup.get(id))
        .map(|frontage| {
            fraction_on_side(
                edge_midpoint(frontage.start, frontage.end),
                site_bbox,
                privacy_side,
            )
        })
        .unwrap_or(0.25);

    let arrival_reservation = reservations
        .iter()
        .find(|reservation| reservation.program_kind == SitePlanProgramKind::ArrivalForecourt);
    let loading_reservation = reservations
        .iter()
        .find(|reservation| reservation.program_kind == SitePlanProgramKind::LoadingZone);
    let landscape_reservation = reservations
        .iter()
        .find(|reservation| reservation.program_kind == SitePlanProgramKind::LandscapeZone);
    let privacy_reservation = reservations
        .iter()
        .find(|reservation| reservation.program_kind == SitePlanProgramKind::PrivacyBuffer);
    let parking_surface_area = parking_allocations
        .iter()
        .filter(|alloc| alloc.parking_mode == ParkingMode::Surface)
        .map(|alloc| alloc.reserved_area_sf)
        .sum::<f64>();

    let arrival_rect = band_rect_for_side(
        public_side,
        site_bbox,
        buildable_bbox,
        public_fraction,
        0.38,
        Some(
            (arrival_reservation
                .map(|x| x.target_area_sf)
                .unwrap_or(650.0)
                / side_length(site_bbox, public_side).max(1.0))
            .max(side_inward_depth(site_bbox, buildable_bbox, public_side).min(14.0)),
        ),
    )
    .unwrap_or(site_bbox.inset(site_bbox.width() * 0.15, site_bbox.height() * 0.15));
    let loading_rect = band_rect_for_side(
        service_side,
        site_bbox,
        buildable_bbox,
        service_fraction,
        0.28,
        Some(
            (loading_reservation
                .map(|x| x.target_area_sf)
                .unwrap_or(a.support.loading_zone_default_sf)
                / side_length(site_bbox, service_side).max(1.0))
            .max(
                input
                    .site_planning
                    .loading
                    .service_yard_depth_ft
                    .unwrap_or(20.0),
            ),
        ),
    )
    .unwrap_or(arrival_rect);
    let fire_rect = band_rect_for_side(
        fire_side,
        site_bbox,
        buildable_bbox,
        fire_fraction,
        0.42,
        Some(
            input
                .site_planning
                .clearance
                .fire_lane_width_ft
                .unwrap_or(20.0)
                .max(side_inward_depth(site_bbox, buildable_bbox, fire_side).min(24.0)),
        ),
    )
    .unwrap_or(arrival_rect);
    let parking_rect = if parking_surface_area > EPS {
        band_rect_for_side(
            parking_side,
            site_bbox,
            buildable_bbox,
            parking_fraction,
            0.45,
            Some((parking_surface_area / side_length(site_bbox, parking_side).max(1.0)).max(18.0)),
        )
    } else {
        None
    };
    let privacy_rect = band_rect_for_side(
        privacy_side,
        site_bbox,
        buildable_bbox,
        privacy_fraction,
        0.32,
        Some(
            input
                .site_planning
                .privacy
                .screening_depth_ft
                .unwrap_or(10.0),
        ),
    )
    .unwrap_or(site_bbox.inset(site_bbox.width() * 0.45, site_bbox.height() * 0.15));
    let opposite_public_side = if matches!(public_side, BboxSide::Top) {
        BboxSide::Bottom
    } else {
        BboxSide::Top
    };
    let landscape_rect = band_rect_for_side(
        opposite_public_side,
        site_bbox,
        buildable_bbox,
        0.5,
        0.7,
        Some(
            (landscape_reservation
                .map(|x| x.target_area_sf)
                .unwrap_or(massing.site_area_sf * 0.1)
                / side_length(site_bbox, opposite_public_side).max(1.0))
            .max(10.0),
        ),
    )
    .unwrap_or(site_bbox.inset(site_bbox.width() * 0.2, site_bbox.height() * 0.2));

    let arrival_anchor_point = arrival_rect.center();
    let building_entry_point = Point2::new(
        footprint_rect.center().x,
        match public_side {
            BboxSide::Bottom => footprint_rect.min_y,
            BboxSide::Top => footprint_rect.max_y,
            BboxSide::Left | BboxSide::Right => footprint_rect.center().y,
        },
    );
    let loading_anchor_point = loading_rect.center();
    let fire_anchor_point = fire_rect.center();
    let parking_anchor_point = parking_rect
        .map(|rect| rect.center())
        .unwrap_or(loading_anchor_point);
    let privacy_anchor_point = privacy_rect.center();
    let landscape_anchor_point = landscape_rect.center();

    let mut anchor_points = vec![
        make_anchor(
            "site_anchor_arrival",
            SitePlanAnchorKind::Arrival,
            arrival_anchor_point,
            public_front_id.clone(),
            Some("reservation_arrival".to_string()),
            true,
            vec![],
        ),
        make_anchor(
            "site_anchor_entry",
            SitePlanAnchorKind::BuildingEntry,
            building_entry_point,
            public_front_id.clone(),
            Some("reservation_residual".to_string()),
            true,
            vec![],
        ),
        make_anchor(
            "site_anchor_loading",
            SitePlanAnchorKind::Loading,
            loading_anchor_point,
            service_front_id.clone(),
            Some("reservation_loading".to_string()),
            true,
            vec![],
        ),
        make_anchor(
            "site_anchor_fire",
            SitePlanAnchorKind::FireAccess,
            fire_anchor_point,
            fire_front_id.clone(),
            Some("reservation_fire".to_string()),
            input.site_planning.fire_access.required,
            vec![],
        ),
        make_anchor(
            "site_anchor_parking",
            SitePlanAnchorKind::ParkingEntry,
            parking_anchor_point,
            parking_front_id.clone(),
            Some("reservation_surface_parking".to_string()),
            input.constraints.parking_mode != ParkingMode::None,
            vec![],
        ),
        make_anchor(
            "site_anchor_privacy",
            SitePlanAnchorKind::Privacy,
            privacy_anchor_point,
            privacy_front_id.clone(),
            Some("reservation_privacy".to_string()),
            false,
            vec![],
        ),
        make_anchor(
            "site_anchor_landscape",
            SitePlanAnchorKind::Landscape,
            landscape_anchor_point,
            None,
            Some("reservation_landscape".to_string()),
            false,
            vec![],
        ),
    ];
    if input.site_planning.frontage.prioritize_multiple_fronts {
        for frontage in frontages
            .iter()
            .filter(|frontage| {
                frontage
                    .active_roles
                    .contains(&SitePlanFrontageRole::PublicEntry)
            })
            .skip(1)
        {
            anchor_points.push(make_anchor(
                &format!("site_anchor_arrival_alt_{:02}", frontage.edge_index),
                SitePlanAnchorKind::Arrival,
                edge_midpoint(frontage.start, frontage.end),
                Some(frontage.frontage_id.clone()),
                Some("reservation_arrival".to_string()),
                false,
                vec!["secondary_public_front".to_string()],
            ));
        }
    }

    let mut segments = vec![
        make_segment(
            "site_segment_public_walk",
            SitePlanSegmentKind::PublicWalk,
            "site_anchor_arrival",
            "site_anchor_entry",
            vec![arrival_anchor_point, building_entry_point],
            input
                .site_planning
                .clearance
                .public_walk_width_ft
                .unwrap_or(10.0),
            true,
            vec![
                "site_zone_arrival".to_string(),
                "site_zone_public_walk".to_string(),
            ],
            vec![],
        ),
        make_segment(
            "site_segment_accessible_walk",
            SitePlanSegmentKind::AccessibleWalk,
            "site_anchor_arrival",
            "site_anchor_entry",
            vec![arrival_anchor_point, building_entry_point],
            input
                .site_planning
                .clearance
                .accessible_walk_width_ft
                .unwrap_or(8.0),
            true,
            vec![
                "site_zone_arrival".to_string(),
                "site_zone_accessible_walk".to_string(),
            ],
            vec![],
        ),
        make_segment(
            "site_segment_service_flow",
            SitePlanSegmentKind::ServiceFlow,
            "site_anchor_loading",
            "site_anchor_entry",
            vec![loading_anchor_point, building_entry_point],
            input
                .site_planning
                .clearance
                .service_path_width_ft
                .unwrap_or(14.0),
            true,
            vec![
                "site_zone_loading".to_string(),
                "site_zone_service".to_string(),
            ],
            vec![],
        ),
        make_segment(
            "site_segment_fire_flow",
            SitePlanSegmentKind::FireFlow,
            "site_anchor_fire",
            "site_anchor_entry",
            vec![fire_anchor_point, building_entry_point],
            input
                .site_planning
                .clearance
                .fire_lane_width_ft
                .unwrap_or(20.0),
            input.site_planning.fire_access.required,
            vec!["site_zone_fire".to_string()],
            vec![],
        ),
    ];
    if parking_rect.is_some() {
        segments.push(make_segment(
            "site_segment_parking_walk",
            SitePlanSegmentKind::ParkingWalk,
            "site_anchor_parking",
            "site_anchor_entry",
            vec![parking_anchor_point, building_entry_point],
            input
                .site_planning
                .clearance
                .parking_walk_width_ft
                .unwrap_or(7.0),
            true,
            vec![
                "site_zone_parking".to_string(),
                "site_zone_parking_walk".to_string(),
            ],
            vec![],
        ));
        segments.push(make_segment(
            "site_segment_drive_aisle",
            SitePlanSegmentKind::DriveAisle,
            "site_anchor_parking",
            "site_anchor_loading",
            vec![parking_anchor_point, loading_anchor_point],
            input
                .site_planning
                .clearance
                .drive_aisle_width_ft
                .unwrap_or(24.0),
            true,
            vec![
                "site_zone_parking".to_string(),
                "site_zone_drive_aisle".to_string(),
            ],
            vec![],
        ));
    }
    if public_side == service_side || public_side == fire_side || service_side == parking_side {
        segments.push(make_segment(
            "site_segment_conflict_crossing",
            SitePlanSegmentKind::ConflictCrossing,
            "site_anchor_loading",
            "site_anchor_arrival",
            vec![loading_anchor_point, arrival_anchor_point],
            input
                .site_planning
                .clearance
                .public_walk_width_ft
                .unwrap_or(10.0)
                .min(
                    input
                        .site_planning
                        .clearance
                        .service_path_width_ft
                        .unwrap_or(14.0),
                ),
            false,
            Vec::new(),
            vec!["shared_front_conflict".to_string()],
        ));
        segments.push(make_segment(
            "site_segment_separation_buffer",
            SitePlanSegmentKind::SeparationBuffer,
            "site_anchor_privacy",
            "site_anchor_loading",
            vec![privacy_anchor_point, loading_anchor_point],
            input
                .site_planning
                .privacy
                .screening_depth_ft
                .unwrap_or(10.0),
            false,
            vec!["site_zone_privacy".to_string()],
            vec!["service_public_separation".to_string()],
        ));
    }

    let public_walk_rect = Rect2::new(
        arrival_anchor_point.x.min(building_entry_point.x)
            - input
                .site_planning
                .clearance
                .public_walk_width_ft
                .unwrap_or(10.0)
                * 0.5,
        arrival_anchor_point.y.min(building_entry_point.y) - 2.0,
        arrival_anchor_point.x.max(building_entry_point.x)
            + input
                .site_planning
                .clearance
                .public_walk_width_ft
                .unwrap_or(10.0)
                * 0.5,
        arrival_anchor_point.y.max(building_entry_point.y) + 2.0,
    );
    let accessible_walk_rect = Rect2::new(
        arrival_anchor_point.x.min(building_entry_point.x)
            - input
                .site_planning
                .clearance
                .accessible_walk_width_ft
                .unwrap_or(8.0)
                * 0.5,
        arrival_anchor_point.y.min(building_entry_point.y) - 1.5,
        arrival_anchor_point.x.max(building_entry_point.x)
            + input
                .site_planning
                .clearance
                .accessible_walk_width_ft
                .unwrap_or(8.0)
                * 0.5,
        arrival_anchor_point.y.max(building_entry_point.y) + 1.5,
    );
    let service_rect = Rect2::new(
        loading_anchor_point.x.min(building_entry_point.x)
            - input
                .site_planning
                .clearance
                .service_path_width_ft
                .unwrap_or(14.0)
                * 0.5,
        loading_anchor_point.y.min(building_entry_point.y) - 2.0,
        loading_anchor_point.x.max(building_entry_point.x)
            + input
                .site_planning
                .clearance
                .service_path_width_ft
                .unwrap_or(14.0)
                * 0.5,
        loading_anchor_point.y.max(building_entry_point.y) + 2.0,
    );
    let cwh_outdoor_pad_area = central_water_heating_pad_outdoor_area_sf(a);
    let parking_walk_rect = parking_rect.map(|_| {
        Rect2::new(
            parking_anchor_point.x.min(building_entry_point.x)
                - input
                    .site_planning
                    .clearance
                    .parking_walk_width_ft
                    .unwrap_or(7.0)
                    * 0.5,
            parking_anchor_point.y.min(building_entry_point.y) - 1.5,
            parking_anchor_point.x.max(building_entry_point.x)
                + input
                    .site_planning
                    .clearance
                    .parking_walk_width_ft
                    .unwrap_or(7.0)
                    * 0.5,
            parking_anchor_point.y.max(building_entry_point.y) + 1.5,
        )
    });
    let drive_aisle_rect = parking_rect.map(|parking_rect| {
        Rect2::new(
            parking_rect.min_x,
            parking_rect.center().y
                - input
                    .site_planning
                    .clearance
                    .drive_aisle_width_ft
                    .unwrap_or(24.0)
                    * 0.5,
            parking_rect.max_x,
            parking_rect.center().y
                + input
                    .site_planning
                    .clearance
                    .drive_aisle_width_ft
                    .unwrap_or(24.0)
                    * 0.5,
        )
    });

    let mut site_zones = vec![
        make_zone(
            "site_zone_arrival",
            SitePlanProgramKind::ArrivalForecourt,
            arrival_rect,
            public_front_id.clone().into_iter().collect(),
            vec![
                "site_anchor_arrival".to_string(),
                "site_anchor_entry".to_string(),
            ],
            vec![
                "site_segment_public_walk".to_string(),
                "site_segment_accessible_walk".to_string(),
            ],
            false,
            vec![],
        ),
        make_zone(
            "site_zone_loading",
            SitePlanProgramKind::LoadingZone,
            loading_rect,
            service_front_id.clone().into_iter().collect(),
            vec!["site_anchor_loading".to_string()],
            vec!["site_segment_service_flow".to_string()],
            false,
            vec![],
        ),
        make_zone(
            "site_zone_service",
            SitePlanProgramKind::ServiceYard,
            service_rect,
            service_front_id.clone().into_iter().collect(),
            vec!["site_anchor_loading".to_string()],
            vec!["site_segment_service_flow".to_string()],
            false,
            vec![],
        ),
        make_zone(
            "site_zone_fire",
            SitePlanProgramKind::FireAccessBand,
            fire_rect,
            fire_front_id.clone().into_iter().collect(),
            vec!["site_anchor_fire".to_string()],
            vec!["site_segment_fire_flow".to_string()],
            false,
            vec![],
        ),
        make_zone(
            "site_zone_public_walk",
            SitePlanProgramKind::PublicWalk,
            public_walk_rect,
            Vec::new(),
            vec![
                "site_anchor_arrival".to_string(),
                "site_anchor_entry".to_string(),
            ],
            vec!["site_segment_public_walk".to_string()],
            false,
            vec![],
        ),
        make_zone(
            "site_zone_accessible_walk",
            SitePlanProgramKind::AccessibleWalk,
            accessible_walk_rect,
            Vec::new(),
            vec![
                "site_anchor_arrival".to_string(),
                "site_anchor_entry".to_string(),
            ],
            vec!["site_segment_accessible_walk".to_string()],
            false,
            vec![],
        ),
        make_zone(
            "site_zone_landscape",
            SitePlanProgramKind::LandscapeZone,
            landscape_rect,
            Vec::new(),
            vec!["site_anchor_landscape".to_string()],
            Vec::new(),
            false,
            landscape_reservation
                .map(|reservation| {
                    vec![format!(
                        "target_area_sf={:.1}",
                        reservation.target_area_sf
                    )]
                })
                .unwrap_or_default(),
        ),
        make_zone(
            "site_zone_privacy",
            SitePlanProgramKind::PrivacyBuffer,
            privacy_rect,
            privacy_front_id.clone().into_iter().collect(),
            vec!["site_anchor_privacy".to_string()],
            Vec::new(),
            false,
            privacy_reservation
                .map(|reservation| {
                    vec![format!(
                        "target_area_sf={:.1}",
                        reservation.target_area_sf
                    )]
                })
                .unwrap_or_default(),
        ),
        make_zone(
            "site_zone_building",
            SitePlanProgramKind::BuildingFootprint,
            footprint_rect,
            Vec::new(),
            vec!["site_anchor_entry".to_string()],
            Vec::new(),
            false,
            vec![format!(
                "upper_footprint_sf={:.1}",
                massing.upper_footprint_sf
            )],
        ),
    ];
    if cwh_outdoor_pad_area > EPS && service_rect.area() > EPS {
        let cwh_outdoor_rect = centered_rect_in_bbox(
            cwh_outdoor_pad_area,
            service_rect,
            a.boh.central_water_heating_pad_outdoor_sum_width_ft.max(1.0)
                / a.boh
                    .central_water_heating_pad_outdoor_sum_depth_ft
                    .max(1.0),
        );
        let mut cwh_outdoor_notes = vec![
            "space_name=Central Water Heating Pad (Outdoor)".to_string(),
            "space_category=Site Support".to_string(),
            "outdoor_service_equipment".to_string(),
            format!("source_area_sf={:.1}", cwh_outdoor_pad_area),
        ];
        if cwh_outdoor_rect.area() + EPS < cwh_outdoor_pad_area {
            cwh_outdoor_notes.push("clamped_to_service_yard".to_string());
        }
        site_zones.push(make_zone(
            "site_zone_cwh_outdoor_pad",
            SitePlanProgramKind::ServiceYard,
            cwh_outdoor_rect,
            service_front_id.clone().into_iter().collect(),
            vec!["site_anchor_loading".to_string()],
            vec!["site_segment_service_flow".to_string()],
            false,
            cwh_outdoor_notes,
        ));
    }
    if input.levels.podium_levels > 0 {
        let podium_rect = centered_rect_in_bbox(
            massing
                .podium_footprint_sf
                .max(footprint_rect.area())
                .min(buildable_bbox.area().max(footprint_rect.area())),
            buildable_bbox,
            if buildable_bbox.height() <= EPS {
                1.0
            } else {
                buildable_bbox.width().max(1.0) / buildable_bbox.height().max(1.0)
            },
        );
        site_zones.push(make_zone(
            "site_zone_podium",
            SitePlanProgramKind::PodiumEnvelope,
            podium_rect,
            Vec::new(),
            vec!["site_anchor_entry".to_string()],
            Vec::new(),
            false,
            vec![format!("podium_levels={}", input.levels.podium_levels)],
        ));
    }
    if let Some(parking_rect) = parking_rect {
        site_zones.push(make_zone(
            "site_zone_parking",
            SitePlanProgramKind::ParkingSurface,
            parking_rect,
            parking_front_id.clone().into_iter().collect(),
            vec!["site_anchor_parking".to_string()],
            vec![
                "site_segment_parking_walk".to_string(),
                "site_segment_drive_aisle".to_string(),
            ],
            false,
            vec![format!(
                "estimated_surface_stalls={}",
                parking_allocations
                    .iter()
                    .filter(|alloc| alloc.parking_mode == ParkingMode::Surface)
                    .map(|alloc| alloc.reserved_stalls)
                    .sum::<u32>()
            )],
        ));
        if let Some(drive_aisle_rect) = drive_aisle_rect {
            site_zones.push(make_zone(
                "site_zone_drive_aisle",
                SitePlanProgramKind::DriveAisle,
                drive_aisle_rect,
                parking_front_id.clone().into_iter().collect(),
                vec!["site_anchor_parking".to_string()],
                vec!["site_segment_drive_aisle".to_string()],
                false,
                vec![],
            ));
        }
        if let Some(parking_walk_rect) = parking_walk_rect {
            site_zones.push(make_zone(
                "site_zone_parking_walk",
                SitePlanProgramKind::ParkingWalk,
                parking_walk_rect,
                parking_front_id.clone().into_iter().collect(),
                vec![
                    "site_anchor_parking".to_string(),
                    "site_anchor_entry".to_string(),
                ],
                vec!["site_segment_parking_walk".to_string()],
                false,
                vec![],
            ));
        }
    }
    if input.levels.below_grade_count > 0
        || parking_allocations.iter().any(|alloc| {
            matches!(
                alloc.parking_mode,
                ParkingMode::Underground | ParkingMode::Podium | ParkingMode::Structured
            ) && alloc.reserved_stalls > 0
        })
    {
        let below_grade_rect = centered_rect_in_bbox(
            buildable_bbox.area() * 0.72,
            buildable_bbox,
            if buildable_bbox.height() <= EPS {
                1.0
            } else {
                buildable_bbox.width().max(1.0) / buildable_bbox.height().max(1.0)
            },
        );
        site_zones.push(make_zone(
            "site_zone_below_grade_parking",
            SitePlanProgramKind::BelowGradeParkingEnvelope,
            below_grade_rect,
            Vec::new(),
            vec!["site_anchor_parking".to_string()],
            Vec::new(),
            false,
            vec!["coarse_volumetric_reservation".to_string()],
        ));
    }

    let used_area = site_zones.iter().map(|zone| zone.area_sf).sum::<f64>();
    let residual_area = (massing.site_area_sf - used_area).max(0.0);
    if residual_area > 120.0 {
        let residual_rect = centered_rect_in_bbox(
            residual_area.min(landscape_rect.area().max(200.0)),
            landscape_rect,
            1.0,
        );
        site_zones.push(make_zone(
            "site_zone_residual",
            SitePlanProgramKind::ResidualDevelopable,
            residual_rect,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            true,
            vec![format!("residual_area_sf={:.1}", residual_area)],
        ));
    }
    if landscape_rect.area() > EPS {
        site_zones.push(make_zone(
            "site_zone_open_space",
            SitePlanProgramKind::OpenSpaceZone,
            centered_rect_in_bbox(landscape_rect.area() * 0.55, landscape_rect, 1.4),
            Vec::new(),
            vec!["site_anchor_landscape".to_string()],
            Vec::new(),
            false,
            vec!["residual_open_space_patch".to_string()],
        ));
    }

    let parking_topology = SiteParkingTopology {
        topology_id: "site_parking_topology".to_string(),
        allocations: parking_allocations.clone(),
        lot_cells: parking_rect
            .map(|parking_rect| {
                vec![SiteParkingLotCell {
                    cell_id: "parking_cell_surface_00".to_string(),
                    polygon: parking_rect.to_polygon(),
                    stall_count_estimate: parking_allocations
                        .iter()
                        .filter(|alloc| alloc.parking_mode == ParkingMode::Surface)
                        .map(|alloc| alloc.reserved_stalls)
                        .sum(),
                    accessible_stalls: parking_allocations
                        .iter()
                        .filter(|alloc| alloc.parking_mode == ParkingMode::Surface)
                        .map(|alloc| alloc.accessible_stalls)
                        .sum(),
                    drive_aisle_segment_ids: vec!["site_segment_drive_aisle".to_string()],
                    parking_walk_segment_ids: vec!["site_segment_parking_walk".to_string()],
                    fragmented: parking_rect.width().min(parking_rect.height()) < 18.0,
                    notes: Vec::new(),
                }]
            })
            .unwrap_or_default(),
        notes: vec![
            format!("required_stalls={}", demand.parking_required_stalls),
            format!("provided_stalls={}", demand.parking_provided_stalls),
        ],
    };

    let turning_radius_ft = input
        .site_planning
        .clearance
        .turning_radius_ft
        .unwrap_or(28.0);
    let maneuver_checks = vec![
        SiteManeuverCheck {
            check_id: "maneuver_parking".to_string(),
            maneuver_class: SitePlanManeuverClass::ParkingIngressEgress,
            status: if let Some(parking_rect) = parking_rect {
                let clearance = parking_rect.width().min(parking_rect.height());
                if clearance >= turning_radius_ft {
                    SitePlanCheckStatus::Pass
                } else if clearance >= turning_radius_ft * 0.8 {
                    SitePlanCheckStatus::Warn
                } else {
                    SitePlanCheckStatus::Fail
                }
            } else {
                SitePlanCheckStatus::Pass
            },
            anchor_ids: vec!["site_anchor_parking".to_string()],
            clearance_ft: parking_rect
                .map(|rect| rect.width().min(rect.height()))
                .unwrap_or(0.0),
            blocking_zone_ids: if parking_rect.is_some() {
                Vec::new()
            } else {
                vec!["site_zone_parking".to_string()]
            },
            notes: Vec::new(),
        },
        SiteManeuverCheck {
            check_id: "maneuver_loading".to_string(),
            maneuver_class: SitePlanManeuverClass::LoadingServiceTurn,
            status: {
                let clearance = loading_rect.width().min(loading_rect.height());
                if clearance >= turning_radius_ft {
                    SitePlanCheckStatus::Pass
                } else if clearance >= turning_radius_ft * 0.8 {
                    SitePlanCheckStatus::Warn
                } else {
                    SitePlanCheckStatus::Fail
                }
            },
            anchor_ids: vec!["site_anchor_loading".to_string()],
            clearance_ft: loading_rect.width().min(loading_rect.height()),
            blocking_zone_ids: Vec::new(),
            notes: Vec::new(),
        },
        SiteManeuverCheck {
            check_id: "maneuver_fire".to_string(),
            maneuver_class: SitePlanManeuverClass::FireAccessTurn,
            status: if input.site_planning.fire_access.required {
                let clearance = fire_rect.width().min(fire_rect.height());
                if clearance >= turning_radius_ft + 4.0 {
                    SitePlanCheckStatus::Pass
                } else if clearance >= (turning_radius_ft + 4.0) * 0.8 {
                    SitePlanCheckStatus::Warn
                } else {
                    SitePlanCheckStatus::Fail
                }
            } else {
                SitePlanCheckStatus::Pass
            },
            anchor_ids: vec!["site_anchor_fire".to_string()],
            clearance_ft: fire_rect.width().min(fire_rect.height()),
            blocking_zone_ids: Vec::new(),
            notes: Vec::new(),
        },
    ];

    let mut clearance_checks = segments
        .iter()
        .map(|segment| {
            let required = match segment.segment_kind {
                SitePlanSegmentKind::PublicWalk => input
                    .site_planning
                    .clearance
                    .public_walk_width_ft
                    .unwrap_or(10.0),
                SitePlanSegmentKind::AccessibleWalk => input
                    .site_planning
                    .clearance
                    .accessible_walk_width_ft
                    .unwrap_or(8.0),
                SitePlanSegmentKind::ServiceFlow => input
                    .site_planning
                    .clearance
                    .service_path_width_ft
                    .unwrap_or(14.0),
                SitePlanSegmentKind::FireFlow => input
                    .site_planning
                    .clearance
                    .fire_lane_width_ft
                    .unwrap_or(20.0),
                SitePlanSegmentKind::ParkingWalk => input
                    .site_planning
                    .clearance
                    .parking_walk_width_ft
                    .unwrap_or(7.0),
                SitePlanSegmentKind::DriveAisle => input
                    .site_planning
                    .clearance
                    .drive_aisle_width_ft
                    .unwrap_or(24.0),
                SitePlanSegmentKind::SeparationBuffer => input
                    .site_planning
                    .privacy
                    .screening_depth_ft
                    .unwrap_or(10.0),
                SitePlanSegmentKind::ConflictCrossing => input
                    .site_planning
                    .clearance
                    .public_walk_width_ft
                    .unwrap_or(10.0),
            };
            let status = if segment.width_ft >= required {
                SitePlanCheckStatus::Pass
            } else if segment.width_ft >= required * 0.85 {
                SitePlanCheckStatus::Warn
            } else {
                SitePlanCheckStatus::Fail
            };
            SiteClearanceCheck {
                check_id: format!("clearance_{}", segment.segment_id),
                segment_id: Some(segment.segment_id.clone()),
                zone_id: None,
                status,
                required_clear_width_ft: required,
                provided_clear_width_ft: segment.width_ft,
                blocking_refs: if matches!(
                    segment.segment_kind,
                    SitePlanSegmentKind::ConflictCrossing
                ) {
                    vec!["shared_front_conflict".to_string()]
                } else {
                    Vec::new()
                },
                notes: if segment.width_ft < required {
                    vec!["width_below_target".to_string()]
                } else {
                    Vec::new()
                },
            }
        })
        .collect::<Vec<_>>();
    if public_side == service_side {
        clearance_checks.push(SiteClearanceCheck {
            check_id: "clearance_service_public_overlap".to_string(),
            segment_id: None,
            zone_id: Some("site_zone_loading".to_string()),
            status: SitePlanCheckStatus::Warn,
            required_clear_width_ft: input
                .site_planning
                .privacy
                .screening_depth_ft
                .unwrap_or(10.0),
            provided_clear_width_ft: 0.0,
            blocking_refs: vec!["shared_front_conflict".to_string()],
            notes: vec!["service_and_public_arrival_share_the_same_frontage".to_string()],
        });
    }

    let outdoor_topology_graph = OutdoorSiteTopologyGraph {
        graph_id: "massing_owned_site_topology".to_string(),
        nodes: site_zones
            .iter()
            .filter_map(|zone| {
                map_zone_kind_to_node_kind(zone.zone_kind).map(|node_kind| OutdoorSiteNode {
                    node_id: format!("node_{}", zone.zone_id),
                    node_kind,
                    area_sf: zone.area_sf,
                    linked_summary_kind: Some(
                        zone_name(zone.zone_kind).to_lowercase().replace(' ', "_"),
                    ),
                    notes: zone.notes.clone(),
                })
            })
            .collect(),
        edges: segments
            .iter()
            .map(|segment| OutdoorSiteEdge {
                edge_id: format!("edge_{}", segment.segment_id),
                from_node_id: segment
                    .linked_zone_ids
                    .first()
                    .map(|zone_id| format!("node_{}", zone_id))
                    .unwrap_or_else(|| "node_site_zone_arrival".to_string()),
                to_node_id: segment
                    .linked_zone_ids
                    .last()
                    .map(|zone_id| format!("node_{}", zone_id))
                    .unwrap_or_else(|| "node_site_zone_arrival".to_string()),
                edge_kind: map_segment_kind_to_edge_kind(segment.segment_kind),
                required: segment.required,
                notes: segment.notes.clone(),
            })
            .collect(),
        notes: vec![
            "layout_massing_owns_all_exterior_site_artifacts".to_string(),
            "layout_space_consumes_bundle_only".to_string(),
        ],
    };

    let concept_volumes = site_zones
        .iter()
        .filter_map(|zone| match zone.zone_kind {
            SitePlanProgramKind::BuildingFootprint => Some(SiteSimpleVolume {
                volume_id: "volume_building".to_string(),
                zone_kind: zone.zone_kind,
                footprint_polygon: zone.polygon.clone(),
                base_level_index: Some(input.levels.below_grade_count),
                top_level_index: Some(massing.story_count.saturating_sub(1)),
                height_ft: massing.story_count as f64 * 10.0,
                below_grade: false,
                notes: Vec::new(),
            }),
            SitePlanProgramKind::PodiumEnvelope if input.levels.podium_levels > 0 => {
                Some(SiteSimpleVolume {
                    volume_id: "volume_podium".to_string(),
                    zone_kind: zone.zone_kind,
                    footprint_polygon: zone.polygon.clone(),
                    base_level_index: Some(input.levels.below_grade_count),
                    top_level_index: Some(
                        input.levels.below_grade_count
                            + input.levels.podium_levels.saturating_sub(1),
                    ),
                    height_ft: input.levels.podium_levels as f64 * 12.0,
                    below_grade: false,
                    notes: Vec::new(),
                })
            }
            SitePlanProgramKind::BelowGradeParkingEnvelope => Some(SiteSimpleVolume {
                volume_id: "volume_below_grade_parking".to_string(),
                zone_kind: zone.zone_kind,
                footprint_polygon: zone.polygon.clone(),
                base_level_index: Some(0),
                top_level_index: Some(input.levels.below_grade_count.saturating_sub(1)),
                height_ft: input.levels.below_grade_count.max(1) as f64 * 10.0,
                below_grade: true,
                notes: vec!["coarse_parking_volume".to_string()],
            }),
            _ => None,
        })
        .collect::<Vec<_>>();

    let parking_penalty = if demand.parking_required_stalls == 0 {
        0.0
    } else {
        (demand
            .parking_required_stalls
            .saturating_sub(demand.parking_provided_stalls) as f64
            / demand.parking_required_stalls.max(1) as f64)
            * 0.22
    };
    let access_penalty = if public_front_id.is_none() || service_front_id.is_none() {
        0.10
    } else {
        0.0
    } + if input.site_planning.fire_access.required && fire_front_id.is_none()
    {
        0.08
    } else {
        0.0
    };
    let privacy_penalty = if public_side == service_side {
        0.06
    } else {
        0.0
    };
    let clearance_penalty = clearance_checks
        .iter()
        .map(|check| match check.status {
            SitePlanCheckStatus::Pass => 0.0,
            SitePlanCheckStatus::Warn => 0.01,
            SitePlanCheckStatus::Fail => 0.04,
        })
        .sum::<f64>()
        + maneuver_checks
            .iter()
            .map(|check| match check.status {
                SitePlanCheckStatus::Pass => 0.0,
                SitePlanCheckStatus::Warn => 0.02,
                SitePlanCheckStatus::Fail => 0.05,
            })
            .sum::<f64>();
    let site_feasibility_penalty = buildable_envelope.issue_codes.len() as f64 * 0.015;
    let far_priority_score = clamp(
        massing.gfa_goal_sf / (input.targets.far_max * massing.site_area_sf).max(1.0),
        0.0,
        1.0,
    );
    let dwelling_priority_score = clamp(
        demand.dwelling_units as f64
            / input
                .targets
                .dwelling_units_cap
                .unwrap_or(demand.dwelling_units.max(1)) as f64,
        0.0,
        1.0,
    );
    let total_score = clamp(
        0.58 * far_priority_score + 0.32 * dwelling_priority_score
            - site_feasibility_penalty
            - access_penalty
            - parking_penalty
            - privacy_penalty
            - clearance_penalty,
        0.0,
        1.0,
    );

    let mut diagnostics = Vec::<ValidationIssue>::new();
    if input.site_planning.california_mode && input.site_planning.overlay.overlay_id.is_none() {
        diagnostics.push(ValidationIssue::warning("local_overlay_missing_or_assumed", "California site-planning mode is active without an explicit local overlay binding; bundle uses fallback exterior controls."));
    }
    if input.constraints.parking_mode != ParkingMode::None
        && !matches!(
            input.site_planning.overlay.binding_mode,
            SiteOverlayBindingMode::Bound
        )
    {
        diagnostics.push(ValidationIssue::warning("parking_ordinance_not_bound", "Parking demand is estimated from assumptions because the local overlay parking ordinance is not fully bound."));
    }
    if public_front_id.is_none() {
        diagnostics.push(ValidationIssue::warning(
            "arrival_unresolved",
            "No viable public arrival frontage was identified for the site-plan bundle.",
        ));
    }
    if service_front_id.is_none() {
        diagnostics.push(ValidationIssue::warning(
            "loading_unresolved",
            "No viable loading/service frontage was identified for the site-plan bundle.",
        ));
    }
    if input.site_planning.fire_access.required && fire_front_id.is_none() {
        diagnostics.push(ValidationIssue::warning("fire_access_unresolved", "Fire access is required but no fire-capable frontage was retained in the site-plan bundle."));
    }
    if demand.parking_provided_stalls < demand.parking_required_stalls {
        diagnostics.push(ValidationIssue::warning(
            "parking_shortfall_estimated",
            "Estimated parking demand exceeds the current coarse parking reservation capacity.",
        ));
    }
    for issue in &buildable_envelope.issue_codes {
        diagnostics.push(ValidationIssue::warning(
            issue,
            "Buildable-envelope normalization emitted a site-planner diagnostic.",
        ));
    }
    if public_side == service_side {
        diagnostics.push(ValidationIssue::warning("service_public_conflict", "Public arrival and service/loading share the same frontage band and require separation handling."));
    }
    if clearance_checks
        .iter()
        .any(|check| matches!(check.status, SitePlanCheckStatus::Fail))
    {
        diagnostics.push(ValidationIssue::warning(
            "exterior_clearance_failure",
            "At least one exterior clearance check failed in the massing-owned site planner.",
        ));
    }
    if maneuver_checks
        .iter()
        .any(|check| matches!(check.status, SitePlanCheckStatus::Fail))
    {
        diagnostics.push(ValidationIssue::warning("maneuver_envelope_failure", "At least one coarse turning/maneuver envelope check failed in the massing-owned site planner."));
    }

    MassingSitePlanBundle {
        bundle_id: format!("site_plan_{:?}_{}F", massing.building_shape, massing.story_count),
        california_site_mode: input.site_planning.california_mode,
        overlay_binding_mode: input.site_planning.overlay.binding_mode,
        overlay_reference: input.site_planning.overlay.overlay_id.clone().or_else(|| input.jurisdiction_profile.local_overlay_id.clone()),
        buildable_envelope,
        frontage_candidates: frontages,
        reservations,
        anchor_points,
        segments,
        site_zones,
        parking_topology,
        maneuver_checks,
        clearance_checks,
        outdoor_topology_graph,
        concept_volumes,
        score_breakdown: SitePlanScoreBreakdown {
            far_priority_score,
            dwelling_priority_score,
            site_feasibility_penalty,
            access_penalty,
            parking_penalty,
            privacy_penalty,
            clearance_penalty,
            total_score,
            notes: vec![
                format!("dwelling_units_estimate={}", demand.dwelling_units),
                format!("avg_unit_area_sf={:.1}", demand.avg_unit_area_sf),
                format!("unit_mix={:?}", demand.unit_counts),
            ],
        },
        diagnostics,
        notes: vec![
            "layout_massing owns all exterior/site artifacts; layout_space only consumes this bundle".to_string(),
            format!("parking_required_stalls={}", demand.parking_required_stalls),
            format!("parking_provided_stalls={}", demand.parking_provided_stalls),
        ],
    }
}

pub fn corridor_width_ft(
    input: &NormalizedInput,
    code: &JurisdictionCodePack,
    a: &AssumptionPack,
) -> f64 {
    let user = match input.constraints.corridor_type {
        CorridorType::Auto | CorridorType::Central | CorridorType::DoubleLoaded => {
            a.corridor_core.preferred_residential_corridor_ft
        }
        CorridorType::SingleLoaded => a.corridor_core.single_loaded_corridor_ft,
        CorridorType::Perimeter => a.corridor_core.perimeter_corridor_ft,
        CorridorType::Internal => a.corridor_core.internal_corridor_ft,
    };
    user.max(code.min_corridor_clear_width_ft)
}

/* ============================== footprint seed ============================ */

pub fn centered_rect_in_bbox(target_area_sf: f64, bbox: Rect2, aspect_ratio: f64) -> Rect2 {
    let ar = aspect_ratio.max(0.25);
    let mut w = (target_area_sf / ar).sqrt();
    let mut d = (target_area_sf * ar).sqrt();
    w = w.min(bbox.width());
    d = d.min(bbox.height());

    Rect2::new(
        bbox.center().x - w * 0.5,
        bbox.center().y - d * 0.5,
        bbox.center().x + w * 0.5,
        bbox.center().y + d * 0.5,
    )
}

pub fn seed_shape_rects(
    shape: BuildingShape,
    target_area_sf: f64,
    site_poly: &[Point2],
    a: &AssumptionPack,
) -> Vec<Rect2> {
    let bbox = bounding_rect(site_poly).inset(a.geometry.site_inset_ft, a.geometry.site_inset_ft);
    let w = bbox.width();
    let h = bbox.height();
    let c = bbox.center();

    let mut rects = Vec::<Rect2>::new();

    match shape {
        BuildingShape::Bar => {
            let depth = a.geometry.daylight_depth_cap_ft.min(h);
            let width = (target_area_sf / depth).min(w);
            rects.push(Rect2::new(
                c.x - width * 0.5,
                c.y - depth * 0.5,
                c.x + width * 0.5,
                c.y + depth * 0.5,
            ));
        }
        BuildingShape::Tower => {
            let side = target_area_sf.sqrt().min(w.min(h));
            rects.push(Rect2::new(
                c.x - side * 0.5,
                c.y - side * 0.5,
                c.x + side * 0.5,
                c.y + side * 0.5,
            ));
        }
        BuildingShape::LShape => {
            let wing = a.geometry.min_wing_width_ft.max(0.22 * w.min(h));
            rects.push(Rect2::new(
                bbox.min_x,
                bbox.min_y,
                bbox.max_x,
                bbox.min_y + wing,
            ));
            rects.push(Rect2::new(
                bbox.min_x,
                bbox.min_y,
                bbox.min_x + wing,
                bbox.max_y,
            ));
        }
        BuildingShape::UShape => {
            let wing = a.geometry.min_wing_width_ft.max(0.18 * w.min(h));
            rects.push(Rect2::new(
                bbox.min_x,
                bbox.min_y,
                bbox.min_x + wing,
                bbox.max_y,
            ));
            rects.push(Rect2::new(
                bbox.max_x - wing,
                bbox.min_y,
                bbox.max_x,
                bbox.max_y,
            ));
            rects.push(Rect2::new(
                bbox.min_x,
                bbox.min_y,
                bbox.max_x,
                bbox.min_y + wing,
            ));
        }
        BuildingShape::OShape => {
            let wing = a.geometry.min_wing_width_ft.max(0.16 * w.min(h));
            rects.push(Rect2::new(
                bbox.min_x,
                bbox.min_y,
                bbox.max_x,
                bbox.min_y + wing,
            ));
            rects.push(Rect2::new(
                bbox.min_x,
                bbox.max_y - wing,
                bbox.max_x,
                bbox.max_y,
            ));
            rects.push(Rect2::new(
                bbox.min_x,
                bbox.min_y + wing,
                bbox.min_x + wing,
                bbox.max_y - wing,
            ));
            rects.push(Rect2::new(
                bbox.max_x - wing,
                bbox.min_y + wing,
                bbox.max_x,
                bbox.max_y - wing,
            ));
        }
        BuildingShape::HShape => {
            let wing = a.geometry.min_wing_width_ft.max(0.18 * w.min(h));
            let bridge = a.geometry.min_wing_width_ft.max(0.14 * w.min(h));
            rects.push(Rect2::new(
                bbox.min_x,
                bbox.min_y,
                bbox.min_x + wing,
                bbox.max_y,
            ));
            rects.push(Rect2::new(
                bbox.max_x - wing,
                bbox.min_y,
                bbox.max_x,
                bbox.max_y,
            ));
            rects.push(Rect2::new(
                c.x - bridge * 0.5,
                bbox.min_y + 0.25 * h,
                c.x + bridge * 0.5,
                bbox.max_y - 0.25 * h,
            ));
        }
        BuildingShape::XShape => {
            let cross = a.geometry.min_wing_width_ft.max(0.20 * w.min(h));
            rects.push(Rect2::new(
                c.x - cross * 0.5,
                bbox.min_y,
                c.x + cross * 0.5,
                bbox.max_y,
            ));
            rects.push(Rect2::new(
                bbox.min_x,
                c.y - cross * 0.5,
                bbox.max_x,
                c.y + cross * 0.5,
            ));
        }
        BuildingShape::Cluster => {
            let cell_w = 0.38 * w;
            let cell_h = 0.34 * h;
            rects.push(Rect2::new(
                bbox.min_x,
                bbox.min_y,
                bbox.min_x + cell_w,
                bbox.min_y + cell_h,
            ));
            rects.push(Rect2::new(
                bbox.max_x - cell_w,
                bbox.min_y,
                bbox.max_x,
                bbox.min_y + cell_h,
            ));
            rects.push(Rect2::new(
                c.x - cell_w * 0.5,
                bbox.max_y - cell_h,
                c.x + cell_w * 0.5,
                bbox.max_y,
            ));
        }
        BuildingShape::FreeForm => {
            let cell = centered_rect_in_bbox(target_area_sf * 0.34, bbox, 1.20);
            rects.push(cell.translate(Point2::new(-0.15 * w, -0.10 * h)));
            rects.push(cell.translate(Point2::new(0.12 * w, -0.02 * h)));
            rects.push(cell.translate(Point2::new(0.00 * w, 0.18 * h)));
        }
        BuildingShape::PerimeterPartial => {
            let wing = a.geometry.min_wing_width_ft.max(0.18 * w.min(h));
            rects.push(Rect2::new(
                bbox.min_x,
                bbox.min_y,
                bbox.max_x,
                bbox.min_y + wing,
            ));
            rects.push(Rect2::new(
                bbox.min_x,
                bbox.min_y,
                bbox.min_x + wing,
                bbox.max_y,
            ));
            rects.push(Rect2::new(
                bbox.max_x - wing,
                bbox.min_y,
                bbox.max_x,
                bbox.max_y * 0.72,
            ));
        }
    }

    rects
}

pub fn rects_area_sum(rects: &[Rect2]) -> f64 {
    rects.iter().map(|r| r.area()).sum()
}

pub fn rects_to_guide_lines(rects: &[Rect2], corridor_type: CorridorType) -> Vec<Line2> {
    let mut out = Vec::<Line2>::new();
    for r in rects {
        let cx = (r.min_x + r.max_x) * 0.5;
        let cy = (r.min_y + r.max_y) * 0.5;
        if r.width() >= r.height() {
            match corridor_type {
                CorridorType::SingleLoaded | CorridorType::DoubleLoaded | CorridorType::Central => {
                    out.push(Line2::new(
                        Point2::new(r.min_x, cy),
                        Point2::new(r.max_x, cy),
                    ));
                }
                CorridorType::Perimeter => {
                    out.push(Line2::new(
                        Point2::new(r.min_x, r.min_y),
                        Point2::new(r.max_x, r.min_y),
                    ));
                }
                CorridorType::Internal => {
                    out.push(Line2::new(
                        Point2::new(cx, r.min_y),
                        Point2::new(cx, r.max_y),
                    ));
                }
                CorridorType::Auto => {
                    out.push(Line2::new(
                        Point2::new(r.min_x, cy),
                        Point2::new(r.max_x, cy),
                    ));
                }
            }
        } else {
            match corridor_type {
                CorridorType::SingleLoaded | CorridorType::DoubleLoaded | CorridorType::Central => {
                    out.push(Line2::new(
                        Point2::new(cx, r.min_y),
                        Point2::new(cx, r.max_y),
                    ));
                }
                CorridorType::Perimeter => {
                    out.push(Line2::new(
                        Point2::new(r.min_x, r.min_y),
                        Point2::new(r.min_x, r.max_y),
                    ));
                }
                CorridorType::Internal => {
                    out.push(Line2::new(
                        Point2::new(r.min_x, cy),
                        Point2::new(r.max_x, cy),
                    ));
                }
                CorridorType::Auto => {
                    out.push(Line2::new(
                        Point2::new(cx, r.min_y),
                        Point2::new(cx, r.max_y),
                    ));
                }
            }
        }
    }
    out
}

/* ========================= override application =========================== */

pub fn apply_override_to_normalized_input(
    input: &mut NormalizedInput,
    ov: &UserOverride,
    vars: &mut VariableBook,
) {
    if !ov.apply {
        return;
    }

    let mut record = |phase: SolvePhase,
                      source: ValueSource,
                      key: &str,
                      derived: Option<ScalarValue>,
                      resolved: ScalarValue,
                      unit: Option<&str>,
                      deps: &[&str]| {
        vars.insert(
            key,
            phase,
            source,
            derived,
            Some(ov.value.clone()),
            resolved,
            unit,
            deps,
        );
    };

    match ov.key.as_str() {
        "levels.count" => {
            if let Some(v) = ov.value.as_u32() {
                let d = input.levels.count;
                input.levels.count = v;
                record(
                    SolvePhase::MassingTargets,
                    ValueSource::FormulaThenOverride,
                    "levels.count",
                    Some(ScalarValue::U32(d)),
                    ScalarValue::U32(v),
                    Some("story"),
                    &["levels.count"],
                );
            }
        }
        "levels.podium_levels" => {
            if let Some(v) = ov.value.as_u32() {
                let d = input.levels.podium_levels;
                input.levels.podium_levels = v;
                record(
                    SolvePhase::MassingTargets,
                    ValueSource::FormulaThenOverride,
                    "levels.podium_levels",
                    Some(ScalarValue::U32(d)),
                    ScalarValue::U32(v),
                    Some("story"),
                    &["levels.podium_levels"],
                );
            }
        }
        "constraints.max_unit_depth_ft" => {
            if let Some(v) = ov.value.as_f64() {
                let d = input.constraints.max_unit_depth_ft;
                input.constraints.max_unit_depth_ft = v;
                record(
                    SolvePhase::ProgramTargets,
                    ValueSource::FormulaThenOverride,
                    "constraints.max_unit_depth_ft",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("ft"),
                    &["constraints.max_unit_depth_ft"],
                );
            }
        }
        "constraints.parking_mode" => {
            if let Some(v) = ov.value.as_text() {
                let d = input.constraints.parking_mode;
                input.constraints.parking_mode = match v {
                    "none" => ParkingMode::None,
                    "surface" => ParkingMode::Surface,
                    "podium" => ParkingMode::Podium,
                    "structured" => ParkingMode::Structured,
                    "underground" => ParkingMode::Underground,
                    "mixed" => ParkingMode::Mixed,
                    _ => ParkingMode::Auto,
                };
                record(
                    SolvePhase::ProgramTargets,
                    ValueSource::FormulaThenOverride,
                    "constraints.parking_mode",
                    Some(ScalarValue::Text(format!("{:?}", d))),
                    ScalarValue::Text(v.to_string()),
                    None,
                    &["constraints.parking_mode"],
                );
            }
        }
        "unit_mix.studio" => {
            if let Some(v) = ov.value.as_f64() {
                let d = input.unit_mix_seed.studio;
                input.unit_mix_seed.studio = v;
                input.unit_mix_seed = input.unit_mix_seed.clone().normalized();
                record(
                    SolvePhase::ProgramTargets,
                    ValueSource::FormulaThenOverride,
                    "unit_mix.studio",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("ratio"),
                    &["unit_mix.*"],
                );
            }
        }
        "unit_mix.one_bedroom" => {
            if let Some(v) = ov.value.as_f64() {
                let d = input.unit_mix_seed.one_bedroom;
                input.unit_mix_seed.one_bedroom = v;
                input.unit_mix_seed = input.unit_mix_seed.clone().normalized();
                record(
                    SolvePhase::ProgramTargets,
                    ValueSource::FormulaThenOverride,
                    "unit_mix.one_bedroom",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("ratio"),
                    &["unit_mix.*"],
                );
            }
        }
        "unit_mix.two_bedroom" => {
            if let Some(v) = ov.value.as_f64() {
                let d = input.unit_mix_seed.two_bedroom;
                input.unit_mix_seed.two_bedroom = v;
                input.unit_mix_seed = input.unit_mix_seed.clone().normalized();
                record(
                    SolvePhase::ProgramTargets,
                    ValueSource::FormulaThenOverride,
                    "unit_mix.two_bedroom",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("ratio"),
                    &["unit_mix.*"],
                );
            }
        }
        "unit_mix.three_bedroom" => {
            if let Some(v) = ov.value.as_f64() {
                let d = input.unit_mix_seed.three_bedroom;
                input.unit_mix_seed.three_bedroom = v;
                input.unit_mix_seed = input.unit_mix_seed.clone().normalized();
                record(
                    SolvePhase::ProgramTargets,
                    ValueSource::FormulaThenOverride,
                    "unit_mix.three_bedroom",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("ratio"),
                    &["unit_mix.*"],
                );
            }
        }
        "residential_features.in_unit_wd.mode" => {
            if let Some(v) = ov.value.as_text() {
                let d = format!("{:?}", input.residential_features.in_unit_wd.mode);
                input.residential_features.in_unit_wd.mode = match v {
                    "all_units" => InUnitWdMode::AllUnits,
                    "none" => InUnitWdMode::None,
                    "partial" => InUnitWdMode::Partial,
                    _ => InUnitWdMode::Auto,
                };
                record(
                    SolvePhase::ProgramTargets,
                    ValueSource::FormulaThenOverride,
                    "residential_features.in_unit_wd.mode",
                    Some(ScalarValue::Text(d)),
                    ScalarValue::Text(v.to_string()),
                    None,
                    &["residential_features.in_unit_wd"],
                );
            }
        }
        "residential_features.in_unit_wd.partial_ratio" => {
            if let Some(v) = ov.value.as_f64() {
                let d = input
                    .residential_features
                    .in_unit_wd
                    .partial_ratio
                    .unwrap_or(0.0);
                input.residential_features.in_unit_wd.partial_ratio = Some(v);
                record(
                    SolvePhase::ProgramTargets,
                    ValueSource::FormulaThenOverride,
                    "residential_features.in_unit_wd.partial_ratio",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("ratio"),
                    &["residential_features.in_unit_wd"],
                );
            }
        }
        "amenities.indoor_target_sf" => {
            if let Some(v) = ov.value.as_f64() {
                let d = input.amenities.indoor_target_sf.unwrap_or(0.0);
                input.amenities.indoor_target_sf = Some(v);
                record(
                    SolvePhase::ProgramTargets,
                    ValueSource::FormulaThenOverride,
                    "amenities.indoor_target_sf",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("sf"),
                    &["amenities"],
                );
            }
        }
        "amenities.outdoor_target_sf" => {
            if let Some(v) = ov.value.as_f64() {
                let d = input.amenities.outdoor_target_sf.unwrap_or(0.0);
                input.amenities.outdoor_target_sf = Some(v);
                record(
                    SolvePhase::ProgramTargets,
                    ValueSource::FormulaThenOverride,
                    "amenities.outdoor_target_sf",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("sf"),
                    &["amenities"],
                );
            }
        }
        "targets.retail_area_sf" => {
            if let Some(v) = ov.value.as_f64() {
                let d = input.targets.retail_area_sf;
                input.targets.retail_area_sf = v;
                record(
                    SolvePhase::MassingTargets,
                    ValueSource::FormulaThenOverride,
                    "targets.retail_area_sf",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("sf"),
                    &["targets.retail_area_sf"],
                );
            }
        }
        "optimization.story_search_max" => {
            if let Some(v) = ov.value.as_u32() {
                let d = input
                    .optimization
                    .story_search_max
                    .unwrap_or(input.levels.count);
                input.optimization.story_search_max = Some(v);
                record(
                    SolvePhase::MassingTargets,
                    ValueSource::FormulaThenOverride,
                    "optimization.story_search_max",
                    Some(ScalarValue::U32(d)),
                    ScalarValue::U32(v),
                    Some("story"),
                    &["optimization.story_search_max"],
                );
            }
        }
        "code_profile_overrides.min_corridor_clear_width_ft" => {
            if let Some(v) = ov.value.as_f64() {
                let mut pack = input.code_profile_overrides.clone().unwrap_or_default();
                let d = pack.min_corridor_clear_width_ft.unwrap_or(0.0);
                pack.min_corridor_clear_width_ft = Some(v);
                input.code_profile_overrides = Some(pack);
                record(
                    SolvePhase::InputNormalization,
                    ValueSource::FormulaThenOverride,
                    "code_profile_overrides.min_corridor_clear_width_ft",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("ft"),
                    &["code_profile_overrides"],
                );
            }
        }
        "code_profile_overrides.retail_sf_per_stall" => {
            if let Some(v) = ov.value.as_f64() {
                let mut pack = input.code_profile_overrides.clone().unwrap_or_default();
                let d = pack.retail_sf_per_stall.unwrap_or(0.0);
                pack.retail_sf_per_stall = Some(v);
                input.code_profile_overrides = Some(pack);
                record(
                    SolvePhase::InputNormalization,
                    ValueSource::FormulaThenOverride,
                    "code_profile_overrides.retail_sf_per_stall",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("sf/stall"),
                    &["code_profile_overrides"],
                );
            }
        }
        "code_profile_overrides.surface_parking_area_ratio" => {
            if let Some(v) = ov.value.as_f64() {
                let mut pack = input.code_profile_overrides.clone().unwrap_or_default();
                let d = pack.surface_parking_area_ratio.unwrap_or(0.0);
                pack.surface_parking_area_ratio = Some(v);
                input.code_profile_overrides = Some(pack);
                record(
                    SolvePhase::InputNormalization,
                    ValueSource::FormulaThenOverride,
                    "code_profile_overrides.surface_parking_area_ratio",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("ratio"),
                    &["code_profile_overrides"],
                );
            }
        }
        "shape_parameters.target_wing_depth_ft" => {
            if let Some(v) = ov.value.as_f64() {
                let mut s = input.shape_parameters.clone().unwrap_or_default();
                let d = s.target_wing_depth_ft.unwrap_or(0.0);
                s.target_wing_depth_ft = Some(v);
                input.shape_parameters = Some(s);
                record(
                    SolvePhase::MassingTargets,
                    ValueSource::FormulaThenOverride,
                    "shape_parameters.target_wing_depth_ft",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("ft"),
                    &["shape_parameters"],
                );
            }
        }
        "shape_parameters.target_courtyard_width_ft" => {
            if let Some(v) = ov.value.as_f64() {
                let mut s = input.shape_parameters.clone().unwrap_or_default();
                let d = s.target_courtyard_width_ft.unwrap_or(0.0);
                s.target_courtyard_width_ft = Some(v);
                input.shape_parameters = Some(s);
                record(
                    SolvePhase::MassingTargets,
                    ValueSource::FormulaThenOverride,
                    "shape_parameters.target_courtyard_width_ft",
                    Some(ScalarValue::F64(d)),
                    ScalarValue::F64(v),
                    Some("ft"),
                    &["shape_parameters"],
                );
            }
        }
        "solver_controls.max_threads" => {
            if let Some(v) = ov.value.as_u32() {
                let mut s = input.solver_controls.clone().unwrap_or_default();
                let d = s.max_threads.unwrap_or(0) as u32;
                s.max_threads = Some(v as usize);
                input.solver_controls = Some(s);
                record(
                    SolvePhase::InputNormalization,
                    ValueSource::FormulaThenOverride,
                    "solver_controls.max_threads",
                    Some(ScalarValue::U32(d)),
                    ScalarValue::U32(v),
                    Some("thread"),
                    &["solver_controls"],
                );
            }
        }
        _ => {
            vars.insert(
                &format!("unsupported_override.{}", ov.key),
                phase_for_override_key(&ov.key),
                ValueSource::UserOverride,
                None,
                Some(ov.value.clone()),
                ov.value.clone(),
                None,
                &[],
            );
        }
    }
}

pub fn parking_gfa_scalar(mode: ParkingMode) -> f64 {
    match mode {
        ParkingMode::None => 1.00,
        ParkingMode::Surface => 0.82,
        ParkingMode::Podium => 0.94,
        ParkingMode::Structured => 0.96,
        ParkingMode::Underground => 0.98,
        ParkingMode::Mixed => 0.93,
        ParkingMode::Auto => 0.90,
    }
}

pub fn weighted_seed_target_area(input: &NormalizedInput, a: &AssumptionPack) -> f64 {
    let studio = a.unit_size_targets_sf.studio.target_sf * input.unit_mix_seed.studio;
    let one = a.unit_size_targets_sf.one_bedroom.target_sf * input.unit_mix_seed.one_bedroom;
    let two = a.unit_size_targets_sf.two_bedroom.target_sf * input.unit_mix_seed.two_bedroom;
    let three = a.unit_size_targets_sf.three_bedroom.target_sf * input.unit_mix_seed.three_bedroom;
    (studio + one + two + three).max(1.0)
}

pub fn select_total_story_count(
    input: &NormalizedInput,
    site_area_sf: f64,
    a: &AssumptionPack,
) -> u32 {
    let plate_seed_target = site_area_sf
        * shape_coverage_ratio(input.building_shape)
        * construction_multiplier(input.building_construction_type);
    let (_, shape_diagnostics) = realize_shape_rects(
        input.building_shape,
        plate_seed_target,
        &input.site_polygon,
        a,
    );
    let plate_seed = shape_diagnostics
        .realized_area_sf
        .max(plate_seed_target * 0.55)
        .min(site_area_sf.max(1.0));
    select_total_story_count_for_shape(
        input,
        site_area_sf,
        plate_seed,
        input.building_shape,
        &shape_diagnostics,
        a,
    )
}

pub fn build_floor_families(
    input: &NormalizedInput,
    massing: &MassingState,
    story_count: u32,
) -> Vec<FloorFamily> {
    let below_grade_count = input
        .levels
        .below_grade_count
        .min(story_count.saturating_sub(1));
    let remaining_above_grade = story_count.saturating_sub(below_grade_count);
    let podium_levels = input
        .levels
        .podium_levels
        .min(remaining_above_grade.saturating_sub(1));
    let below_grade_end = below_grade_count;
    let podium_start = below_grade_end;
    let podium_end = podium_start + podium_levels;
    let typical_start = podium_end;

    let mut families = Vec::<FloorFamily>::new();
    if below_grade_count > 0 {
        families.push(FloorFamily {
            family_id: "below_grade".to_string(),
            level_indices: (0..below_grade_end).collect(),
            is_typical: false,
            area_budget_sf: massing.site_area_sf * 0.85,
            uses_upper_footprint: false,
            family_role: "below_grade".to_string(),
            bound_block_ids: Vec::new(),
            binding_notes: Vec::new(),
        });
    }

    if podium_levels > 0 {
        families.push(FloorFamily {
            family_id: "podium".to_string(),
            level_indices: (podium_start..podium_end).collect(),
            is_typical: false,
            area_budget_sf: massing.podium_footprint_sf,
            uses_upper_footprint: false,
            family_role: "podium".to_string(),
            bound_block_ids: Vec::new(),
            binding_notes: Vec::new(),
        });
    }

    let typical_levels = (typical_start..story_count).collect::<Vec<_>>();
    if !typical_levels.is_empty() {
        families.push(FloorFamily {
            family_id: "typical_residential".to_string(),
            level_indices: typical_levels,
            is_typical: input.levels.typical_floor && input.vertical_rules.repeat_typical_floors,
            area_budget_sf: massing.upper_footprint_sf.max(massing.footprint_seed_sf),
            uses_upper_footprint: true,
            family_role: "tower".to_string(),
            bound_block_ids: Vec::new(),
            binding_notes: Vec::new(),
        });
    }

    families
}

fn stable_block_rects(rects: &[Rect2]) -> Vec<Rect2> {
    let mut out = rects.to_vec();
    out.sort_by(|a, b| {
        let ac = a.center();
        let bc = b.center();
        ac.y.partial_cmp(&bc.y)
            .unwrap_or(Ordering::Equal)
            .then_with(|| ac.x.partial_cmp(&bc.x).unwrap_or(Ordering::Equal))
            .then_with(|| a.area().partial_cmp(&b.area()).unwrap_or(Ordering::Equal))
    });
    out
}

fn rect_touches_bbox_edge(rect: Rect2, bbox: Rect2) -> bool {
    (rect.min_x - bbox.min_x).abs() <= 1.0
        || (rect.max_x - bbox.max_x).abs() <= 1.0
        || (rect.min_y - bbox.min_y).abs() <= 1.0
        || (rect.max_y - bbox.max_y).abs() <= 1.0
}

fn concept_block_role(
    shape: BuildingShape,
    rects: &[Rect2],
    rect: Rect2,
    ordinal_index: usize,
) -> ConceptBlockRole {
    if rects.len() <= 1 {
        return ConceptBlockRole::PrimaryMass;
    }

    let bbox = rects.iter().copied().fold(rects[0], |acc, next| {
        Rect2::new(
            acc.min_x.min(next.min_x),
            acc.min_y.min(next.min_y),
            acc.max_x.max(next.max_x),
            acc.max_y.max(next.max_y),
        )
    });
    let center = bbox.center();
    let is_central = rect.center().distance_to(center) <= 0.18 * bbox.width().max(bbox.height());
    let touches_edge = rect_touches_bbox_edge(rect, bbox);

    match shape {
        BuildingShape::HShape if is_central => ConceptBlockRole::Bridge,
        BuildingShape::XShape if is_central => ConceptBlockRole::ServiceSpine,
        BuildingShape::Cluster | BuildingShape::FreeForm if ordinal_index > 0 => {
            ConceptBlockRole::Branch
        }
        BuildingShape::LShape
        | BuildingShape::UShape
        | BuildingShape::OShape
        | BuildingShape::PerimeterPartial
            if touches_edge =>
        {
            ConceptBlockRole::CourtyardEdge
        }
        _ if ordinal_index == 0 => ConceptBlockRole::PrimaryMass,
        _ => ConceptBlockRole::Wing,
    }
}

fn family_block_keep_count(
    family: &FloorFamily,
    massing: &MassingState,
    total_blocks: usize,
) -> usize {
    if total_blocks <= 1 {
        return total_blocks;
    }
    if family.family_id != "typical_residential" || massing.podium_footprint_sf <= EPS {
        return total_blocks;
    }
    let ratio = family.area_budget_sf / massing.podium_footprint_sf.max(1.0);
    if ratio < 0.58 {
        total_blocks.saturating_sub(2).max(1)
    } else if ratio < 0.80 {
        total_blocks.saturating_sub(1).max(1)
    } else {
        total_blocks
    }
}

fn build_concept_block_contract(
    input: &NormalizedInput,
    families: &[FloorFamily],
    massing: &MassingState,
    a: &AssumptionPack,
    building_shape: BuildingShape,
) -> (
    Vec<FloorFamily>,
    Vec<ConceptBlock>,
    Vec<ConceptFamilyBlockBinding>,
) {
    if families.is_empty() {
        return (Vec::new(), Vec::new(), Vec::new());
    }

    let logical_target_area = families
        .iter()
        .map(|family| family.area_budget_sf)
        .fold(massing.footprint_seed_sf.max(1.0), f64::max);
    let logical_rects = stable_block_rects(
        &realize_shape_rects(building_shape, logical_target_area, &input.site_polygon, a).0,
    );

    let bbox = if logical_rects.is_empty() {
        Rect2::new(0.0, 0.0, 0.0, 0.0)
    } else {
        logical_rects
            .iter()
            .copied()
            .fold(logical_rects[0], |acc, next| {
                Rect2::new(
                    acc.min_x.min(next.min_x),
                    acc.min_y.min(next.min_y),
                    acc.max_x.max(next.max_x),
                    acc.max_y.max(next.max_y),
                )
            })
    };
    let center = bbox.center();
    let mut centrality = logical_rects
        .iter()
        .enumerate()
        .map(|(idx, rect)| (idx, rect.center().distance_to(center)))
        .collect::<Vec<_>>();
    centrality.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });

    let concept_blocks = logical_rects
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, rect)| {
            let primary_role = concept_block_role(building_shape, &logical_rects, rect, idx);
            let is_central =
                rect.center().distance_to(center) <= 0.18 * bbox.width().max(bbox.height());
            ConceptBlock {
                block_id: format!("block_{:02}", idx + 1),
                source_shape: building_shape,
                ordinal_index: idx,
                primary_role,
                service_capable: is_central || idx == 0,
                courtyard_edge: primary_role == ConceptBlockRole::CourtyardEdge,
                branch_like: primary_role == ConceptBlockRole::Branch,
                notes: vec![format!("logical_area_sf={:.1}", rect.area())],
            }
        })
        .collect::<Vec<_>>();

    let mut previous_by_block = BTreeMap::<String, (bool, f64, String)>::new();
    let mut bindings = Vec::<ConceptFamilyBlockBinding>::new();
    let mut updated_families = Vec::<FloorFamily>::new();

    for family in families {
        let family_rects = stable_block_rects(
            &realize_shape_rects(
                building_shape,
                family.area_budget_sf.max(1.0),
                &input.site_polygon,
                a,
            )
            .0,
        );
        let keep_count = family_block_keep_count(family, massing, concept_blocks.len());
        let active_indices = centrality
            .iter()
            .take(keep_count.min(concept_blocks.len()))
            .map(|(idx, _)| *idx)
            .collect::<Vec<_>>();
        let mut family_active_block_ids = Vec::<String>::new();
        let mut family_notes = vec![
            format!("family_role={}", family.family_role),
            format!("keep_count={}", keep_count),
        ];

        for (idx, block) in concept_blocks.iter().enumerate() {
            let rect = family_rects
                .get(idx)
                .copied()
                .or_else(|| logical_rects.get(idx).copied())
                .unwrap_or(Rect2::new(0.0, 0.0, 0.0, 0.0));
            let active = active_indices.iter().any(|active_idx| *active_idx == idx);
            let area_sf = if active { rect.area() } else { 0.0 };
            let previous = previous_by_block.get(&block.block_id).cloned();
            let transition_kind = match previous {
                Some((true, prev_area, _)) if !active => BlockBindingTransitionKind::Suppressed,
                Some((false, _, _)) if active => BlockBindingTransitionKind::Introduced,
                Some((true, prev_area, _)) if active && area_sf < prev_area * 0.95 => {
                    BlockBindingTransitionKind::Tapered
                }
                Some((true, prev_area, _)) if active && area_sf > prev_area * 1.05 => {
                    BlockBindingTransitionKind::Expanded
                }
                Some((true, _, _)) if active => BlockBindingTransitionKind::Persistent,
                Some((true, _, _)) => BlockBindingTransitionKind::Persistent,
                Some((false, _, _)) => BlockBindingTransitionKind::Suppressed,
                None if active => BlockBindingTransitionKind::Introduced,
                None => BlockBindingTransitionKind::Suppressed,
            };

            if active {
                family_active_block_ids.push(block.block_id.clone());
            }
            if !active {
                family_notes.push(format!("suppressed={}", block.block_id));
            }

            bindings.push(ConceptFamilyBlockBinding {
                binding_id: format!("bind_{}_{}", family.family_id, block.block_id),
                family_id: family.family_id.clone(),
                block_id: block.block_id.clone(),
                level_indices: family.level_indices.clone(),
                rect_seed: rect,
                polygon: rect.to_polygon(),
                area_sf,
                active,
                transition_kind,
                inherited_from_family_id: previous.map(|(_, _, prev_family_id)| prev_family_id),
                notes: vec![
                    format!("family_role={}", family.family_role),
                    format!("active={}", active),
                ],
            });

            previous_by_block.insert(
                block.block_id.clone(),
                (active, area_sf.max(rect.area()), family.family_id.clone()),
            );
        }

        let mut family_clone = family.clone();
        family_clone.bound_block_ids = family_active_block_ids;
        family_clone.binding_notes = family_notes;
        updated_families.push(family_clone);
    }

    (updated_families, concept_blocks, bindings)
}

fn shape_corridor_core_compatibility(
    input: &NormalizedInput,
    candidate: &CandidateCase,
) -> (bool, Vec<String>) {
    let corridor_forced = input.constraints.corridor_type != CorridorType::Auto;
    let core_forced = input.constraints.core_strategy != CoreStrategy::Auto;
    let mut notes = Vec::<String>::new();

    if candidate.shape_diagnostics.clipping_loss_ratio > 0.40 {
        notes.push("parcel_fit_loss_excessive".to_string());
        return (false, notes);
    }
    if candidate.shape_diagnostics.fragment_count > 8 {
        notes.push("parcel_fit_fragmentation_excessive".to_string());
        return (false, notes);
    }
    if !corridor_forced {
        match candidate.building_shape {
            BuildingShape::Tower
                if matches!(
                    candidate.corridor_type,
                    CorridorType::SingleLoaded | CorridorType::Perimeter
                ) =>
            {
                notes.push("tower_prefers_internal_or_double_loaded_corridor".to_string());
                return (false, notes);
            }
            BuildingShape::HShape | BuildingShape::XShape | BuildingShape::Cluster
                if matches!(candidate.corridor_type, CorridorType::Perimeter) =>
            {
                notes.push("branched_shape_rejects_perimeter_corridor".to_string());
                return (false, notes);
            }
            BuildingShape::OShape | BuildingShape::PerimeterPartial
                if matches!(candidate.corridor_type, CorridorType::SingleLoaded) =>
            {
                notes.push("courtyard_shape_prefers_double_loaded_or_perimeter_access".to_string());
                return (false, notes);
            }
            _ => {}
        }
    }
    if !core_forced {
        match candidate.building_shape {
            BuildingShape::Tower
                if matches!(candidate.core_strategy, CoreStrategy::Distributed) =>
            {
                notes.push("tower_prefers_centralized_core".to_string());
                return (false, notes);
            }
            BuildingShape::OShape | BuildingShape::UShape | BuildingShape::PerimeterPartial
                if matches!(candidate.core_strategy, CoreStrategy::Corner) =>
            {
                notes.push("courtyard_shape_rejects_corner_core".to_string());
                return (false, notes);
            }
            BuildingShape::Cluster | BuildingShape::FreeForm
                if matches!(candidate.core_strategy, CoreStrategy::Central) =>
            {
                notes.push("distributed_shape_prefers_multiple_or_distributed_cores".to_string());
                return (false, notes);
            }
            _ => {}
        }
    }
    if matches!(candidate.construction_case, ConstructionCase::LowRiseTypeV)
        && candidate.building_shape == BuildingShape::Tower
        && candidate.story_count > 6
    {
        notes.push("low_rise_type_v_tower_story_count_too_high".to_string());
        return (false, notes);
    }
    if candidate.shape_diagnostics.clipping_loss_ratio > 0.18 {
        notes.push("parcel_fit_loss_warn".to_string());
    }
    if candidate.shape_diagnostics.fragment_count > 4 {
        notes.push("fragment_count_warn".to_string());
    }
    (true, notes)
}

fn truncate_candidates_round_robin_by_shape(
    candidates: Vec<CandidateCase>,
    limit: usize,
) -> Vec<CandidateCase> {
    if candidates.len() <= limit || limit == 0 {
        return candidates.into_iter().take(limit).collect();
    }

    let mut shape_order = Vec::<BuildingShape>::new();
    let mut grouped = Vec::<VecDeque<CandidateCase>>::new();
    for candidate in candidates {
        if let Some(idx) = shape_order
            .iter()
            .position(|shape| *shape == candidate.building_shape)
        {
            grouped[idx].push_back(candidate);
        } else {
            shape_order.push(candidate.building_shape);
            let mut bucket = VecDeque::<CandidateCase>::new();
            bucket.push_back(candidate);
            grouped.push(bucket);
        }
    }

    if grouped.len() <= 1 {
        return grouped
            .into_iter()
            .flat_map(|bucket| bucket.into_iter())
            .take(limit)
            .collect();
    }

    let mut selected = Vec::<CandidateCase>::with_capacity(limit);
    while selected.len() < limit {
        let mut made_progress = false;
        for bucket in grouped.iter_mut() {
            if selected.len() >= limit {
                break;
            }
            if let Some(candidate) = bucket.pop_front() {
                selected.push(candidate);
                made_progress = true;
            }
        }
        if !made_progress {
            break;
        }
    }
    selected
}

pub fn build_candidate_cases(state: &EngineState) -> Vec<CandidateCase> {
    let input = state.normalized.as_ref().unwrap();
    let corridor_candidates = default_corridor_candidates(input);
    let core_candidates = default_core_candidates(input);
    let shapes = derive_shape_candidates(input);

    let mut out = Vec::<CandidateCase>::new();
    for shape in shapes {
        let baseline_massing = solve_massing_for_shape(input, &state.assumptions, shape, None);
        let mut story_range = vec![baseline_massing.story_count];
        if input.optimization.allow_story_override {
            let lo = input
                .optimization
                .story_search_min
                .unwrap_or(input.levels.count.max(1));
            let hi = input
                .optimization
                .story_search_max
                .unwrap_or((input.levels.count + 2).max(lo));
            story_range = (lo..=hi).collect();
        }

        for story_count in story_range {
            let candidate_massing = if story_count == baseline_massing.story_count {
                baseline_massing.clone()
            } else {
                solve_massing_for_shape(input, &state.assumptions, shape, Some(story_count))
            };
            for corridor_type in corridor_candidates.iter().copied() {
                for core_strategy in core_candidates.iter().copied() {
                    let mut candidate = CandidateCase {
                        candidate_id: String::new(),
                        building_shape: shape,
                        story_count,
                        podium_levels: input.levels.podium_levels,
                        corridor_type,
                        core_strategy,
                        shape_case: candidate_massing.shape_case,
                        construction_case: candidate_massing.construction_case,
                        footprint_seed_sf: candidate_massing.footprint_seed_sf,
                        podium_footprint_sf: candidate_massing.podium_footprint_sf,
                        upper_footprint_sf: candidate_massing.upper_footprint_sf,
                        shape_diagnostics: candidate_massing.shape_diagnostics.clone(),
                        site_plan_bundle: candidate_massing.site_plan_bundle.clone(),
                        notes: vec![
                            format!("shape={:?}", shape),
                            format!("story_count={}", story_count),
                            format!(
                                "parcel_fit_ratio={:.3}",
                                candidate_massing.shape_diagnostics.parcel_fit_ratio
                            ),
                            format!(
                                "site_plan_score={:.3}",
                                candidate_massing
                                    .site_plan_bundle
                                    .score_breakdown
                                    .total_score
                            ),
                        ],
                    };
                    candidate.notes.extend(
                        candidate_massing
                            .site_plan_bundle
                            .diagnostics
                            .iter()
                            .map(|issue| format!("site_plan_diag={}", issue.code)),
                    );
                    let (compatible, compatibility_notes) =
                        shape_corridor_core_compatibility(input, &candidate);
                    candidate.notes.extend(compatibility_notes);
                    if compatible || !prune_incompatible_candidates(input) {
                        out.push(candidate);
                    }
                }
            }
        }
    }
    if let Some(ctrl) = &input.solver_controls {
        if let Some(limit) = ctrl.candidate_count {
            if out.len() > limit {
                out = truncate_candidates_round_robin_by_shape(out, limit);
            }
        }
    }
    for (idx, candidate) in out.iter_mut().enumerate() {
        candidate.candidate_id = format!("cand_{:03}", idx + 1);
    }
    out
}

fn preliminary_concept_score(input: &NormalizedInput, candidate: &CandidateCase) -> f64 {
    let story_delta = candidate.story_count.abs_diff(input.levels.count) as f64;
    let story_alignment = clamp(
        1.0 - story_delta / candidate.story_count.max(input.levels.count).max(1) as f64,
        0.0,
        1.0,
    );
    let corridor_score = match candidate.corridor_type {
        CorridorType::DoubleLoaded => 1.0,
        CorridorType::Central => 0.96,
        CorridorType::SingleLoaded => 0.88,
        CorridorType::Perimeter => 0.84,
        CorridorType::Internal => 0.80,
        CorridorType::Auto => 0.78,
    };
    let core_score = match candidate.core_strategy {
        CoreStrategy::Central => 1.0,
        CoreStrategy::Corner => 0.92,
        CoreStrategy::Multiple => 0.90,
        CoreStrategy::Distributed => 0.88,
        CoreStrategy::Auto => 0.86,
    };
    let footprint_alignment = clamp(
        candidate.upper_footprint_sf / candidate.footprint_seed_sf.max(1.0),
        0.0,
        1.0,
    );
    let parcel_fit_score = clamp(candidate.shape_diagnostics.parcel_fit_ratio, 0.0, 1.0);
    let shape_score = clamp(shape_coverage_ratio(candidate.building_shape), 0.0, 1.0);
    let objective_bias = match input.optimization.objective {
        OptimizationObjective::MaximizeFar => {
            0.35 * footprint_alignment
                + 0.25 * story_alignment
                + 0.25 * parcel_fit_score
                + 0.15 * shape_score
        }
        OptimizationObjective::MaximizeDwellingUnits => {
            0.30 * footprint_alignment + 0.35 * story_alignment + 0.25 * parcel_fit_score
        }
        OptimizationObjective::MaximizeBalancedYield => {
            0.30 * footprint_alignment
                + 0.25 * story_alignment
                + 0.25 * parcel_fit_score
                + 0.20 * shape_score
        }
    };

    objective_bias + 0.12 * corridor_score + 0.10 * core_score
        - 0.08 * candidate.shape_diagnostics.clipping_loss_ratio
}

pub fn generate_concept_options(
    input: &LayoutInput,
    assumptions: &AssumptionPack,
) -> Vec<ConceptOption> {
    let mut state = EngineState::new(input.clone(), assumptions.clone());
    solve_input_normalization(&mut state);
    solve_massing_targets(&mut state);
    generate_concept_options_from_state(&state)
}

pub fn generate_concept_options_from_state(state: &EngineState) -> Vec<ConceptOption> {
    let input = match state.normalized.as_ref() {
        Some(input) => input,
        None => return Vec::new(),
    };
    let massing = match state.massing.as_ref() {
        Some(massing) => massing,
        None => return Vec::new(),
    };

    let mut options = build_candidate_cases(state)
        .into_iter()
        .map(|candidate| {
            let candidate_massing = solve_massing_for_shape(
                input,
                &state.assumptions,
                candidate.building_shape,
                Some(candidate.story_count),
            );
            let story_count = candidate.story_count;
            let floor_families = build_floor_families(input, &candidate_massing, story_count);
            let (floor_families, concept_blocks, family_block_bindings) =
                build_concept_block_contract(
                    input,
                    &floor_families,
                    &candidate_massing,
                    &state.assumptions,
                    candidate.building_shape,
                );
            let concept_score = preliminary_concept_score(input, &candidate)
                + 0.35 * candidate.site_plan_bundle.score_breakdown.total_score;
            ConceptOption {
                concept_id: candidate.candidate_id.clone(),
                story_count,
                below_grade_count: input
                    .levels
                    .below_grade_count
                    .min(story_count.saturating_sub(1)),
                podium_levels: candidate.podium_levels,
                building_shape: candidate.building_shape,
                construction_case: candidate.construction_case,
                shape_case: candidate.shape_case,
                corridor_type_seed: candidate.corridor_type,
                core_strategy_seed: candidate.core_strategy,
                footprint_seed_sf: candidate.footprint_seed_sf,
                podium_footprint_sf: candidate.podium_footprint_sf,
                upper_footprint_sf: candidate.upper_footprint_sf,
                floor_families,
                concept_blocks: concept_blocks.clone(),
                family_block_bindings: family_block_bindings.clone(),
                envelope: ConceptEnvelope {
                    building_shape: candidate.building_shape,
                    site_area_sf: candidate_massing.site_area_sf,
                    site_perimeter_ft: candidate_massing.site_perimeter_ft,
                    gfa_goal_sf: candidate_massing.gfa_goal_sf,
                    footprint_seed_sf: candidate.footprint_seed_sf,
                    podium_footprint_sf: candidate.podium_footprint_sf,
                    upper_footprint_sf: candidate.upper_footprint_sf,
                    shape_diagnostics: candidate.shape_diagnostics.clone(),
                    concept_blocks: concept_blocks.clone(),
                    family_block_bindings: family_block_bindings.clone(),
                },
                site_plan_bundle: candidate.site_plan_bundle.clone(),
                flex_budget: LayoutFlexBudget::default(),
                concept_score,
                shape_diagnostics: candidate.shape_diagnostics.clone(),
                notes: candidate
                    .notes
                    .iter()
                    .cloned()
                    .chain([
                        format!("corridor={:?}", candidate.corridor_type),
                        format!("core={:?}", candidate.core_strategy),
                        format!("story_count={}", story_count),
                        format!("logical_blocks={}", concept_blocks.len()),
                    ])
                    .collect(),
            }
        })
        .collect::<Vec<_>>();

    options.sort_by(|a, b| {
        b.concept_score
            .partial_cmp(&a.concept_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.concept_id.cmp(&b.concept_id))
    });
    options
}

pub fn select_default_concept(options: &[ConceptOption], input: &LayoutInput) -> ConceptOption {
    let preferred_corridor = input.constraints.corridor_type;
    let preferred_core = input.constraints.core_strategy;
    options
        .iter()
        .max_by(|a, b| {
            let a_pref = usize::from(
                preferred_corridor == CorridorType::Auto
                    || a.corridor_type_seed == preferred_corridor,
            ) + usize::from(
                preferred_core == CoreStrategy::Auto || a.core_strategy_seed == preferred_core,
            );
            let b_pref = usize::from(
                preferred_corridor == CorridorType::Auto
                    || b.corridor_type_seed == preferred_corridor,
            ) + usize::from(
                preferred_core == CoreStrategy::Auto || b.core_strategy_seed == preferred_core,
            );
            a_pref
                .cmp(&b_pref)
                .then_with(|| {
                    a.concept_score
                        .partial_cmp(&b.concept_score)
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| b.concept_id.cmp(&a.concept_id))
        })
        .cloned()
        .expect("select_default_concept requires at least one concept option")
}

pub fn recompute_concept_from_override(
    state: &mut EngineState,
    user_override: UserOverride,
) -> Vec<ConceptOption> {
    let dirty = phase_for_override_key(&user_override.key);
    state.input.user_overrides.push(user_override);
    state.invalidate_from(dirty);
    if state.normalized.is_none() {
        solve_input_normalization(state);
    }
    if state.massing.is_none() {
        solve_massing_targets(state);
    }
    generate_concept_options_from_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_concept_options_smoke() {
        let input = crate::sample_repo_integration_input().to_core();
        let options = generate_concept_options(&input, &AssumptionPack::default());
        assert!(!options.is_empty());
    }

    #[test]
    fn generate_concept_options_searches_multiple_shapes_deterministically() {
        let mut input = crate::sample_repo_integration_input().to_core();
        input.constraints.corridor_type = CorridorType::Auto;
        input.constraints.core_strategy = CoreStrategy::Auto;
        input.solver_controls = Some(SolverControlsInput {
            max_threads: Some(2),
            candidate_count: Some(48),
            floor_family_parallel: Some(false),
            candidate_parallel: Some(false),
            shape_search_enabled: Some(true),
            shape_search_limit: Some(4),
            prune_incompatible_candidates: Some(true),
        });

        let options_a = generate_concept_options(&input, &AssumptionPack::default());
        let options_b = generate_concept_options(&input, &AssumptionPack::default());
        let mut shapes = options_a
            .iter()
            .map(|option| format!("{:?}", option.building_shape))
            .collect::<Vec<_>>();
        shapes.sort();
        shapes.dedup();

        assert!(shapes.len() > 1);
        assert_eq!(
            options_a
                .iter()
                .map(|option| (
                    option.concept_id.clone(),
                    option.building_shape,
                    option.story_count
                ))
                .collect::<Vec<_>>(),
            options_b
                .iter()
                .map(|option| (
                    option.concept_id.clone(),
                    option.building_shape,
                    option.story_count
                ))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn build_candidate_cases_honors_single_shape_disable_and_pruning() {
        let assumptions = AssumptionPack::default();
        let mut input = crate::sample_repo_integration_input().to_core();
        input.building_shape = BuildingShape::Tower;
        input.constraints.corridor_type = CorridorType::Auto;
        input.constraints.core_strategy = CoreStrategy::Auto;
        input.solver_controls = Some(SolverControlsInput {
            max_threads: Some(2),
            candidate_count: Some(40),
            floor_family_parallel: Some(false),
            candidate_parallel: Some(false),
            shape_search_enabled: Some(false),
            shape_search_limit: Some(1),
            prune_incompatible_candidates: Some(true),
        });

        let mut state = EngineState::new(input.clone(), assumptions);
        solve_input_normalization(&mut state);
        solve_massing_targets(&mut state);
        let candidates = build_candidate_cases(&state);

        assert!(!candidates.is_empty());
        assert!(candidates
            .iter()
            .all(|candidate| candidate.building_shape == BuildingShape::Tower));
        assert!(!candidates.iter().any(|candidate| {
            candidate.building_shape == BuildingShape::Tower
                && matches!(
                    candidate.corridor_type,
                    CorridorType::SingleLoaded | CorridorType::Perimeter
                )
        }));
    }

    #[test]
    fn override_levels_count_marks_massing_targets_dirty() {
        assert_eq!(
            phase_for_override_key("levels.count"),
            SolvePhase::MassingTargets
        );
    }

    #[test]
    fn override_max_unit_depth_marks_program_targets_dirty() {
        assert_eq!(
            phase_for_override_key("constraints.max_unit_depth_ft"),
            SolvePhase::ProgramTargets
        );
    }

    #[test]
    fn concept_options_expose_massing_owned_site_plan_bundle_before_detail_solve() {
        let input = crate::sample_repo_integration_input().to_core();
        let options = generate_concept_options(&input, &AssumptionPack::default());
        let bundle = &options[0].site_plan_bundle;

        assert!(!bundle.frontage_candidates.is_empty());
        assert!(bundle
            .site_zones
            .iter()
            .any(|zone| zone.zone_kind == SitePlanProgramKind::ArrivalForecourt));
        assert!(bundle
            .site_zones
            .iter()
            .any(|zone| zone.zone_kind == SitePlanProgramKind::LoadingZone));
        assert!(bundle
            .outdoor_topology_graph
            .nodes
            .iter()
            .any(|node| node.node_kind == OutdoorSiteNodeKind::ArrivalEntry));
        assert!(bundle
            .outdoor_topology_graph
            .nodes
            .iter()
            .any(|node| node.node_kind == OutdoorSiteNodeKind::LoadingZone));
        assert!(bundle
            .notes
            .iter()
            .any(|note| note.contains("layout_massing owns all exterior/site artifacts")));
    }

    #[test]
    fn site_plan_bundle_reserves_outdoor_cwh_pad_when_indoor_room_is_disabled() {
        let input = crate::sample_repo_integration_input().to_core();
        let mut assumptions = AssumptionPack::default();
        assumptions.boh.central_water_heating_room_indoor_enabled = false;
        assumptions.boh.central_water_heating_pad_outdoor_enabled = true;

        let options = generate_concept_options(&input, &assumptions);
        let bundle = &options[0].site_plan_bundle;
        let cwh_pad_zone = bundle
            .site_zones
            .iter()
            .find(|zone| zone.zone_id == "site_zone_cwh_outdoor_pad")
            .expect("outdoor CWH pad should be reserved in the massing-owned site plan bundle");

        assert_eq!(cwh_pad_zone.zone_kind, SitePlanProgramKind::ServiceYard);
        assert!(cwh_pad_zone
            .notes
            .iter()
            .any(|note| note == "space_name=Central Water Heating Pad (Outdoor)"));
        assert!(cwh_pad_zone.area_sf > 0.0);
        assert!(
            cwh_pad_zone.area_sf
                <= central_water_heating_pad_outdoor_area_sf(&assumptions) + 1.0e-6
        );
        assert!(cwh_pad_zone
            .notes
            .iter()
            .any(|note| note.starts_with("source_area_sf=")));
        assert!(bundle
            .outdoor_topology_graph
            .nodes
            .iter()
            .any(|node| node
                .notes
                .iter()
                .any(|note| note == "space_name=Central Water Heating Pad (Outdoor)")));
    }

    #[test]
    fn site_plan_bundle_supports_multiple_public_fronts_and_overlay_fallbacks() {
        let mut input = crate::sample_repo_integration_input().to_core();
        input.jurisdiction_profile.local_overlay_id = None;
        input.site_planning.california_mode = true;
        input.site_planning.overlay.binding_mode = SiteOverlayBindingMode::Missing;
        input.site_planning.overlay.overlay_id = None;
        input.site_planning.frontage.frontage_edge_indices = vec![0, 1];
        input.site_planning.frontage.entry_edge_indices = vec![0, 1];
        input.site_planning.frontage.service_access_edge_indices = vec![2];
        input.site_planning.frontage.fire_access_edge_indices = vec![1, 2];
        input.site_planning.frontage.prioritize_multiple_fronts = true;
        input.site_planning.buildable.no_build_edge_indices = vec![3];

        let options = generate_concept_options(&input, &AssumptionPack::default());
        let bundle = &options[0].site_plan_bundle;

        let public_front_count = bundle
            .frontage_candidates
            .iter()
            .filter(|frontage| {
                frontage
                    .active_roles
                    .contains(&SitePlanFrontageRole::PublicEntry)
            })
            .count();

        assert!(public_front_count >= 2);
        assert!(bundle
            .diagnostics
            .iter()
            .any(|issue| issue.code == "local_overlay_missing_or_assumed"));
    }

    #[test]
    fn site_plan_bundle_tracks_parking_shortfall_and_stays_deterministic() {
        let mut input = crate::sample_repo_integration_input().to_core();
        input.site_polygon = vec![
            Point2::new(0.0, 0.0),
            Point2::new(80.0, 0.0),
            Point2::new(80.0, 60.0),
            Point2::new(0.0, 60.0),
        ];
        input.levels.count = 6;
        input.constraints.parking_mode = ParkingMode::Surface;
        input.targets.far_max = 4.5;
        input.optimization.far_fill_target = 0.95;
        input.site_planning.parking.provided_stalls_cap = Some(6);

        let options_a = generate_concept_options(&input, &AssumptionPack::default());
        let options_b = generate_concept_options(&input, &AssumptionPack::default());
        let bundle_a = &options_a[0].site_plan_bundle;
        let bundle_b = &options_b[0].site_plan_bundle;

        assert!(bundle_a
            .diagnostics
            .iter()
            .any(|issue| issue.code == "parking_shortfall_estimated"));
        assert_eq!(
            bundle_a
                .site_zones
                .iter()
                .map(|zone| zone.zone_id.clone())
                .collect::<Vec<_>>(),
            bundle_b
                .site_zones
                .iter()
                .map(|zone| zone.zone_id.clone())
                .collect::<Vec<_>>()
        );
        assert!(bundle_a.concept_volumes.iter().all(|volume| matches!(
            volume.zone_kind,
            SitePlanProgramKind::BuildingFootprint
                | SitePlanProgramKind::PodiumEnvelope
                | SitePlanProgramKind::BelowGradeParkingEnvelope
        )));
    }

    #[test]
    fn massing_carries_preliminary_budget_and_respects_unit_density_cap() {
        let assumptions = AssumptionPack::default();
        let mut input = crate::sample_repo_integration_input().to_core();
        input.levels.count = 10;
        input.levels.podium_levels = 1;
        input.targets.dwelling_units_cap = Some(140);
        input.constraints.parking_mode = ParkingMode::Podium;

        let mut state = EngineState::new(input.clone(), assumptions.clone());
        solve_input_normalization(&mut state);
        solve_massing_targets(&mut state);

        let normalized = state.normalized.as_ref().unwrap();
        let massing = state.massing.as_ref().unwrap();
        let gfa_cap = preliminary_gfa_from_dwelling_units(
            normalized.targets.dwelling_units_cap.unwrap(),
            normalized.targets.retail_area_sf,
            massing.story_count,
            &normalized.unit_mix_seed,
            normalized,
            &assumptions,
        );

        assert!(massing.preliminary_area_budget.residential_area_sf > 0.0);
        assert!(massing.gfa_goal_sf <= gfa_cap + 1.0e-6);
    }
}
