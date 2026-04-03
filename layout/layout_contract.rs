use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub const EPS: f64 = 1.0e-6;

/* =============================================================================
Apartment layout engine substrate
--------------------------------------------------------------------------------
This contract file keeps the inherited geometric/program substrate and adds a
progressive topology wrapper:
1. coarse morphology seed
2. backbone / zoning graph
3. constrained allocation
4. targeted exactification

Legacy version-note clutter is intentionally omitted from this header.
============================================================================= */

/* ================================ geometry ================================= */

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

impl Point2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    pub fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }

    pub fn scale(self, k: f64) -> Self {
        Self::new(self.x * k, self.y * k)
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn normalized(self) -> Self {
        let l = self.len();
        if l <= EPS {
            Self::new(1.0, 0.0)
        } else {
            self.scale(1.0 / l)
        }
    }

    pub fn perp(self) -> Self {
        Self::new(-self.y, self.x)
    }

    pub fn distance_to(self, other: Self) -> f64 {
        self.sub(other).len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect2 {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Rect2 {
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    pub fn width(self) -> f64 {
        (self.max_x - self.min_x).max(0.0)
    }

    pub fn height(self) -> f64 {
        (self.max_y - self.min_y).max(0.0)
    }

    pub fn area(self) -> f64 {
        self.width() * self.height()
    }

    pub fn center(self) -> Point2 {
        Point2::new(
            (self.min_x + self.max_x) * 0.5,
            (self.min_y + self.max_y) * 0.5,
        )
    }

    pub fn inset(self, dx: f64, dy: f64) -> Self {
        Self::new(
            self.min_x + dx,
            self.min_y + dy,
            self.max_x - dx,
            self.max_y - dy,
        )
    }

    pub fn translate(self, d: Point2) -> Self {
        Self::new(
            self.min_x + d.x,
            self.min_y + d.y,
            self.max_x + d.x,
            self.max_y + d.y,
        )
    }

    pub fn to_polygon(self) -> Vec<Point2> {
        vec![
            Point2::new(self.min_x, self.min_y),
            Point2::new(self.max_x, self.min_y),
            Point2::new(self.max_x, self.max_y),
            Point2::new(self.min_x, self.max_y),
        ]
    }

    pub fn contains(self, p: Point2) -> bool {
        p.x >= self.min_x - EPS
            && p.x <= self.max_x + EPS
            && p.y >= self.min_y - EPS
            && p.y <= self.max_y + EPS
    }

    pub fn intersects(self, other: Self) -> bool {
        !(self.max_x <= other.min_x + EPS
            || self.min_x >= other.max_x - EPS
            || self.max_y <= other.min_y + EPS
            || self.min_y >= other.max_y - EPS)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line2 {
    pub a: Point2,
    pub b: Point2,
}

impl Line2 {
    pub fn new(a: Point2, b: Point2) -> Self {
        Self { a, b }
    }

    pub fn length(self) -> f64 {
        self.a.distance_to(self.b)
    }

    pub fn dir(self) -> Point2 {
        self.b.sub(self.a).normalized()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LocalPoint {
    pub s: f64,
    pub t: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct LocalFrame {
    pub origin: Point2,
    pub u: Point2,
    pub v: Point2,
}

impl LocalFrame {
    pub fn new(origin: Point2, u: Point2) -> Self {
        let u_n = u.normalized();
        Self {
            origin,
            u: u_n,
            v: u_n.perp(),
        }
    }

    pub fn project(&self, p: Point2) -> LocalPoint {
        let d = p.sub(self.origin);
        LocalPoint {
            s: d.dot(self.u),
            t: d.dot(self.v),
        }
    }

    pub fn unproject(&self, p: LocalPoint) -> Point2 {
        self.origin.add(self.u.scale(p.s)).add(self.v.scale(p.t))
    }
}

pub fn bounding_rect(poly: &[Point2]) -> Rect2 {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for p in poly {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }

    Rect2::new(min_x, min_y, max_x, max_y)
}

pub fn polygon_area_sf(poly: &[Point2]) -> f64 {
    if poly.len() < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        s += a.x * b.y - b.x * a.y;
    }
    0.5 * s.abs()
}

pub fn polygon_perimeter_ft(poly: &[Point2]) -> f64 {
    if poly.len() < 2 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        s += a.distance_to(b);
    }
    s
}

pub fn poly_centroid(poly: &[Point2]) -> Point2 {
    if poly.is_empty() {
        return Point2::new(0.0, 0.0);
    }
    let mut sx = 0.0;
    let mut sy = 0.0;
    for p in poly {
        sx += p.x;
        sy += p.y;
    }
    Point2::new(sx / poly.len() as f64, sy / poly.len() as f64)
}

pub fn snap_ft(x: f64, g: f64) -> f64 {
    if g <= EPS {
        x
    } else {
        (x / g).round() * g
    }
}

pub fn clamp(x: f64, lo: f64, hi: f64) -> f64 {
    x.max(lo).min(hi)
}

pub fn lerp(x: f64, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    if (x1 - x0).abs() <= EPS {
        y0
    } else if x <= x0 {
        y0
    } else if x >= x1 {
        y1
    } else {
        let t = (x - x0) / (x1 - x0);
        y0 + t * (y1 - y0)
    }
}

pub fn interp_u32(x: f64, x0: f64, y0: f64, x1: f64, y1: f64) -> u32 {
    lerp(x, x0, y0, x1, y1).round().max(0.0) as u32
}

/* ================================= enums ================================== */

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SolvePhase {
    InputNormalization,
    MassingTargets,
    ProgramTargets,
    VerticalSystem,
    FloorZoning,
    SpaceLayout,
    OutputAssembly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueSource {
    Assumption,
    Formula,
    UserOverride,
    AssumptionThenOverride,
    FormulaThenOverride,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolutionStatus {
    Feasible,
    FeasibleWithAssumptions,
    FeasibleWithManualReview,
    Infeasible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseCodeEdition {
    Ibc2021,
    Ibc2024,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateAmendmentProfile {
    None,
    Ca2022Title24,
    Ca2025Title24,
    Fl2023Fbc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingShape {
    Bar,
    LShape,
    UShape,
    OShape,
    HShape,
    Tower,
    XShape,
    Cluster,
    FreeForm,
    PerimeterPartial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingConstructionType {
    TypeV,
    TypeIII,
    TypeVOverI,
    TypeIIIOverI,
    TypeI,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorridorType {
    Auto,
    Central,
    SingleLoaded,
    DoubleLoaded,
    Perimeter,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreStrategy {
    Auto,
    Central,
    Corner,
    Multiple,
    Distributed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParkingMode {
    Auto,
    None,
    Surface,
    Podium,
    Structured,
    Underground,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InUnitWdMode {
    Auto,
    AllUnits,
    None,
    Partial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmenityStrategy {
    MinCode,
    Balanced,
    Premium,
    UserSelected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationObjective {
    MaximizeFar,
    MaximizeDwellingUnits,
    MaximizeBalancedYield,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructionCase {
    LowRiseTypeV,
    MidRiseTypeIII,
    PodiumTypeVOverI,
    PodiumTypeIIIOverI,
    HighRiseTypeI,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeCase {
    Linear,
    Courtyard,
    Branched,
    Distributed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitType {
    Studio,
    OneBedroom,
    TwoBedroom,
    ThreeBedroom,
}

impl UnitType {
    pub fn all() -> [UnitType; 4] {
        [
            UnitType::Studio,
            UnitType::OneBedroom,
            UnitType::TwoBedroom,
            UnitType::ThreeBedroom,
        ]
    }

    pub fn as_key(self) -> &'static str {
        match self {
            UnitType::Studio => "studio",
            UnitType::OneBedroom => "one_bedroom",
            UnitType::TwoBedroom => "two_bedroom",
            UnitType::ThreeBedroom => "three_bedroom",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScalarValue {
    Bool(bool),
    U32(u32),
    F64(f64),
    Text(String),
}

impl ScalarValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ScalarValue::F64(x) => Some(*x),
            ScalarValue::U32(x) => Some(*x as f64),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            ScalarValue::U32(x) => Some(*x),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ScalarValue::Bool(x) => Some(*x),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            ScalarValue::Text(x) => Some(x.as_str()),
            _ => None,
        }
    }
}

/* ================================= input ================================== */

#[derive(Debug, Clone)]
pub struct JurisdictionProfile {
    pub base_code_edition: BaseCodeEdition,
    pub state_amendment_profile: StateAmendmentProfile,
    pub local_overlay_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LevelsInput {
    pub count: u32,
    pub below_grade_count: u32,
    pub podium_levels: u32,
    pub typical_floor: bool,
}

#[derive(Debug, Clone)]
pub struct UnitMix {
    pub studio: f64,
    pub one_bedroom: f64,
    pub two_bedroom: f64,
    pub three_bedroom: f64,
}

impl UnitMix {
    pub fn sum(&self) -> f64 {
        self.studio + self.one_bedroom + self.two_bedroom + self.three_bedroom
    }

    pub fn normalized(mut self) -> Self {
        let s = self.sum();
        if s.abs() <= EPS {
            self.one_bedroom = 1.0;
            return self;
        }
        self.studio /= s;
        self.one_bedroom /= s;
        self.two_bedroom /= s;
        self.three_bedroom /= s;
        self
    }

    pub fn get(&self, t: UnitType) -> f64 {
        match t {
            UnitType::Studio => self.studio,
            UnitType::OneBedroom => self.one_bedroom,
            UnitType::TwoBedroom => self.two_bedroom,
            UnitType::ThreeBedroom => self.three_bedroom,
        }
    }

    pub fn set(&mut self, t: UnitType, v: f64) {
        match t {
            UnitType::Studio => self.studio = v,
            UnitType::OneBedroom => self.one_bedroom = v,
            UnitType::TwoBedroom => self.two_bedroom = v,
            UnitType::ThreeBedroom => self.three_bedroom = v,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnitSizeTarget {
    pub min_sf: f64,
    pub target_sf: f64,
    pub max_sf: f64,
}

#[derive(Debug, Clone)]
pub struct UnitSizeTargets {
    pub studio: UnitSizeTarget,
    pub one_bedroom: UnitSizeTarget,
    pub two_bedroom: UnitSizeTarget,
    pub three_bedroom: UnitSizeTarget,
}

impl UnitSizeTargets {
    pub fn get(&self, t: UnitType) -> &UnitSizeTarget {
        match t {
            UnitType::Studio => &self.studio,
            UnitType::OneBedroom => &self.one_bedroom,
            UnitType::TwoBedroom => &self.two_bedroom,
            UnitType::ThreeBedroom => &self.three_bedroom,
        }
    }

    pub fn get_mut(&mut self, t: UnitType) -> &mut UnitSizeTarget {
        match t {
            UnitType::Studio => &mut self.studio,
            UnitType::OneBedroom => &mut self.one_bedroom,
            UnitType::TwoBedroom => &mut self.two_bedroom,
            UnitType::ThreeBedroom => &mut self.three_bedroom,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConstraintsInput {
    pub max_unit_depth_ft: f64,
    pub max_unit_depth_input: Option<String>,
    pub min_daylight: bool,
    pub corridor_type: CorridorType,
    pub core_strategy: CoreStrategy,
    pub parking_mode: ParkingMode,
}

#[derive(Debug, Clone)]
pub struct InUnitWdInput {
    pub mode: InUnitWdMode,
    pub partial_ratio: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ResidentialFeaturesInput {
    pub in_unit_wd: InUnitWdInput,
}

#[derive(Debug, Clone)]
pub struct VerticalRulesInput {
    pub core_alignment: bool,
    pub shaft_alignment: bool,
    pub repeat_typical_floors: bool,
}

#[derive(Debug, Clone)]
pub struct AmenitiesInput {
    pub strategy: AmenityStrategy,
    pub indoor_target_sf: Option<f64>,
    pub outdoor_target_sf: Option<f64>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TargetsInput {
    pub far_max: f64,
    pub dwelling_units_cap: Option<u32>,
    pub gfa_cap_sf: Option<f64>,
    pub retail_area_sf: f64,
}

#[derive(Debug, Clone)]
pub struct OptimizationWeights {
    pub far: f64,
    pub dwelling_units: f64,
    pub yield_: f64,
    pub amenity: f64,
    pub repeatability: f64,
}

#[derive(Debug, Clone)]
pub struct OptimizationInput {
    pub objective: OptimizationObjective,
    pub allow_story_override: bool,
    pub story_search_min: Option<u32>,
    pub story_search_max: Option<u32>,
    pub allow_unit_mix_rebalance: bool,
    pub allow_unit_size_rebalance: bool,
    pub far_fill_target: f64,
    pub weights: OptimizationWeights,
}

#[derive(Debug, Clone)]
pub struct UserOverride {
    pub key: String,
    pub apply: bool,
    pub value: ScalarValue,
}

#[derive(Debug, Clone, Default)]
pub struct CodeProfileOverridesInput {
    pub min_corridor_clear_width_ft: Option<f64>,
    pub occupant_load_factor_residential_sf_per_occ: Option<f64>,
    pub exit_access_travel_sprinklered_ft: Option<f64>,
    pub stair_width_per_occ_in: Option<f64>,
    pub stair_width_min_in: Option<f64>,
    pub retail_sf_per_stall: Option<f64>,
    pub surface_parking_area_ratio: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct ShapeParametersInput {
    pub target_wing_depth_ft: Option<f64>,
    pub target_courtyard_width_ft: Option<f64>,
    pub tower_core_area_ratio: Option<f64>,
    pub perimeter_bar_width_ft: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct SolverControlsInput {
    pub max_threads: Option<usize>,
    pub candidate_count: Option<usize>,
    pub floor_family_parallel: Option<bool>,
    pub candidate_parallel: Option<bool>,
    pub shape_search_enabled: Option<bool>,
    pub shape_search_limit: Option<usize>,
    pub prune_incompatible_candidates: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteOverlayBindingMode {
    Bound,
    Assumed,
    Missing,
}

#[derive(Debug, Clone)]
pub struct SiteOverlayReferenceInput {
    pub binding_mode: SiteOverlayBindingMode,
    pub overlay_id: Option<String>,
    pub source_tag: Option<String>,
}

impl Default for SiteOverlayReferenceInput {
    fn default() -> Self {
        Self {
            binding_mode: SiteOverlayBindingMode::Missing,
            overlay_id: None,
            source_tag: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SiteFrontageControlsInput {
    pub frontage_edge_indices: Vec<usize>,
    pub entry_edge_indices: Vec<usize>,
    pub fire_access_edge_indices: Vec<usize>,
    pub service_access_edge_indices: Vec<usize>,
    pub privacy_edge_indices: Vec<usize>,
    pub allow_service_on_public_front: bool,
    pub prioritize_multiple_fronts: bool,
}

#[derive(Debug, Clone)]
pub struct SiteBuildableEnvelopeInput {
    pub edge_setbacks_ft: Vec<f64>,
    pub default_front_setback_ft: f64,
    pub default_side_setback_ft: f64,
    pub default_rear_setback_ft: f64,
    pub no_build_edge_indices: Vec<usize>,
    pub min_buildable_area_sf: f64,
    pub setback_snap_ft: f64,
}

impl Default for SiteBuildableEnvelopeInput {
    fn default() -> Self {
        Self {
            edge_setbacks_ft: Vec::new(),
            default_front_setback_ft: 12.0,
            default_side_setback_ft: 8.0,
            default_rear_setback_ft: 10.0,
            no_build_edge_indices: Vec::new(),
            min_buildable_area_sf: 12_000.0,
            setback_snap_ft: 2.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SiteLoadingControlsInput {
    pub prefer_shared_service_and_fire_access: bool,
    pub loading_zone_area_sf: Option<f64>,
    pub service_yard_depth_ft: Option<f64>,
}

impl Default for SiteLoadingControlsInput {
    fn default() -> Self {
        Self {
            prefer_shared_service_and_fire_access: true,
            loading_zone_area_sf: None,
            service_yard_depth_ft: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SiteFireAccessControlsInput {
    pub required: bool,
    pub preferred_clear_width_ft: Option<f64>,
    pub apparatus_reach_depth_ft: Option<f64>,
}

impl Default for SiteFireAccessControlsInput {
    fn default() -> Self {
        Self {
            required: true,
            preferred_clear_width_ft: None,
            apparatus_reach_depth_ft: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SitePrivacyControlsInput {
    pub screening_depth_ft: Option<f64>,
    pub quiet_edge_depth_ft: Option<f64>,
}

impl Default for SitePrivacyControlsInput {
    fn default() -> Self {
        Self {
            screening_depth_ft: None,
            quiet_edge_depth_ft: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SiteClearanceReserveInput {
    pub public_walk_width_ft: Option<f64>,
    pub accessible_walk_width_ft: Option<f64>,
    pub service_path_width_ft: Option<f64>,
    pub fire_lane_width_ft: Option<f64>,
    pub parking_walk_width_ft: Option<f64>,
    pub drive_aisle_width_ft: Option<f64>,
    pub turning_radius_ft: Option<f64>,
}

impl Default for SiteClearanceReserveInput {
    fn default() -> Self {
        Self {
            public_walk_width_ft: None,
            accessible_walk_width_ft: None,
            service_path_width_ft: None,
            fire_lane_width_ft: None,
            parking_walk_width_ft: None,
            drive_aisle_width_ft: None,
            turning_radius_ft: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SiteParkingControlsInput {
    pub guest_stalls_per_du: Option<f64>,
    pub tdm_reduction_ratio: f64,
    pub accessible_stall_ratio: Option<f64>,
    pub provided_stalls_cap: Option<u32>,
    pub allow_shared_retail_parking: bool,
}

impl Default for SiteParkingControlsInput {
    fn default() -> Self {
        Self {
            guest_stalls_per_du: None,
            tdm_reduction_ratio: 0.0,
            accessible_stall_ratio: None,
            provided_stalls_cap: None,
            allow_shared_retail_parking: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SitePlanningInput {
    pub enabled: bool,
    pub california_mode: bool,
    pub frontage: SiteFrontageControlsInput,
    pub buildable: SiteBuildableEnvelopeInput,
    pub loading: SiteLoadingControlsInput,
    pub fire_access: SiteFireAccessControlsInput,
    pub privacy: SitePrivacyControlsInput,
    pub clearance: SiteClearanceReserveInput,
    pub parking: SiteParkingControlsInput,
    pub overlay: SiteOverlayReferenceInput,
}

impl Default for SitePlanningInput {
    fn default() -> Self {
        Self {
            enabled: true,
            california_mode: true,
            frontage: SiteFrontageControlsInput::default(),
            buildable: SiteBuildableEnvelopeInput::default(),
            loading: SiteLoadingControlsInput::default(),
            fire_access: SiteFireAccessControlsInput::default(),
            privacy: SitePrivacyControlsInput::default(),
            clearance: SiteClearanceReserveInput::default(),
            parking: SiteParkingControlsInput::default(),
            overlay: SiteOverlayReferenceInput::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OverrideTraceRecord {
    pub key: String,
    pub apply: bool,
    pub override_value: ScalarValue,
    pub earliest_recompute_phase: SolvePhase,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct LayoutInput {
    pub jurisdiction_profile: JurisdictionProfile,
    pub building_shape: BuildingShape,
    pub building_construction_type: BuildingConstructionType,
    pub site_polygon: Vec<Point2>,
    pub site_planning: SitePlanningInput,
    pub levels: LevelsInput,
    pub unit_mix: UnitMix,
    pub constraints: ConstraintsInput,
    pub residential_features: ResidentialFeaturesInput,
    pub vertical_rules: VerticalRulesInput,
    pub amenities: AmenitiesInput,
    pub targets: TargetsInput,
    pub optimization: OptimizationInput,
    pub code_profile_overrides: Option<CodeProfileOverridesInput>,
    pub shape_parameters: Option<ShapeParametersInput>,
    pub solver_controls: Option<SolverControlsInput>,
    pub user_overrides: Vec<UserOverride>,
}

#[derive(Debug, Clone)]
pub struct NormalizedInput {
    pub jurisdiction_profile: JurisdictionProfile,
    pub building_shape: BuildingShape,
    pub building_construction_type: BuildingConstructionType,
    pub site_polygon: Vec<Point2>,
    pub site_planning: SitePlanningInput,
    pub levels: LevelsInput,
    pub unit_mix_seed: UnitMix,
    pub constraints: ConstraintsInput,
    pub residential_features: ResidentialFeaturesInput,
    pub vertical_rules: VerticalRulesInput,
    pub amenities: AmenitiesInput,
    pub targets: TargetsInput,
    pub optimization: OptimizationInput,
    pub code_profile_overrides: Option<CodeProfileOverridesInput>,
    pub shape_parameters: Option<ShapeParametersInput>,
    pub solver_controls: Option<SolverControlsInput>,
}

impl LayoutInput {
    pub fn validate(&self) -> Vec<ValidationIssue> {
        let mut out = Vec::<ValidationIssue>::new();

        if self.site_polygon.len() < 3 {
            out.push(ValidationIssue::error(
                "site_polygon_too_small",
                "site_polygon must contain at least 3 points",
            ));
        }
        if !self.site_planning.buildable.edge_setbacks_ft.is_empty()
            && self.site_planning.buildable.edge_setbacks_ft.len() != self.site_polygon.len()
        {
            out.push(ValidationIssue::error(
                "site_planning_edge_setback_count_mismatch",
                "site_planning.buildable.edge_setbacks_ft length must equal the number of site polygon edges",
            ));
        }
        if self.site_planning.buildable.min_buildable_area_sf <= 0.0 {
            out.push(ValidationIssue::error(
                "site_planning_min_buildable_area_nonpositive",
                "site_planning.buildable.min_buildable_area_sf must be > 0",
            ));
        }
        if self.levels.count == 0 {
            out.push(ValidationIssue::error(
                "level_count_zero",
                "levels.count must be greater than 0",
            ));
        }
        if self.constraints.max_unit_depth_ft <= 0.0 {
            out.push(ValidationIssue::error(
                "max_unit_depth_nonpositive",
                "constraints.max_unit_depth_ft must be > 0",
            ));
        }

        let s = self.unit_mix.sum();
        if (s - 1.0).abs() > 1.0e-6 {
            out.push(ValidationIssue::error(
                "unit_mix_not_one",
                "unit_mix ratios must sum to 1.0",
            ));
        }

        match self.building_construction_type {
            BuildingConstructionType::TypeVOverI | BuildingConstructionType::TypeIIIOverI => {
                if self.levels.podium_levels < 1 {
                    out.push(ValidationIssue::error(
                        "podium_required",
                        "podium_levels must be >= 1 for podium construction types",
                    ));
                }
            }
            _ => {
                if self.levels.podium_levels != 0 {
                    out.push(ValidationIssue::error(
                        "podium_forbidden",
                        "podium_levels must be 0 for non-podium construction types",
                    ));
                }
            }
        }

        if self.levels.podium_levels >= self.levels.count {
            out.push(ValidationIssue::error(
                "podium_ge_count",
                "podium_levels must be smaller than levels.count",
            ));
        }

        if self.levels.below_grade_count >= self.levels.count {
            out.push(ValidationIssue::error(
                "below_grade_ge_count",
                "levels.below_grade_count must be smaller than levels.count",
            ));
        }

        if self.levels.below_grade_count + self.levels.podium_levels >= self.levels.count {
            out.push(ValidationIssue::error(
                "stack_partition_invalid",
                "levels.below_grade_count + levels.podium_levels must leave at least one upper residential floor",
            ));
        }

        if self.optimization.far_fill_target <= 0.0 || self.optimization.far_fill_target > 1.0 {
            out.push(ValidationIssue::error(
                "far_fill_target_out_of_range",
                "optimization.far_fill_target must be in (0, 1]",
            ));
        }

        if let (Some(a), Some(b)) = (
            self.optimization.story_search_min,
            self.optimization.story_search_max,
        ) {
            if a > b {
                out.push(ValidationIssue::error(
                    "story_range_invalid",
                    "optimization.story_search_min must be <= optimization.story_search_max",
                ));
            }
        }

        match self.residential_features.in_unit_wd.mode {
            InUnitWdMode::Partial => {
                if self.residential_features.in_unit_wd.partial_ratio.is_none() {
                    out.push(ValidationIssue::error(
                        "partial_ratio_missing",
                        "in_unit_wd.partial_ratio is required when mode = partial",
                    ));
                }
            }
            _ => {}
        }

        out
    }
}

/* ================================ tracing ================================= */

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub code: String,
    pub message: String,
}

impl ValidationIssue {
    pub fn error(code: &str, message: &str) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            code: code.to_string(),
            message: message.to_string(),
        }
    }

    pub fn warning(code: &str, message: &str) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

pub fn validate_unit_size_targets_sf(
    unit_size_targets_sf: &UnitSizeTargets,
) -> Vec<ValidationIssue> {
    let mut out = Vec::<ValidationIssue>::new();
    for t in UnitType::all() {
        let band = unit_size_targets_sf.get(t);
        if !(band.min_sf <= band.target_sf && band.target_sf <= band.max_sf) {
            out.push(ValidationIssue::error(
                "unit_size_target_order_invalid",
                &format!(
                    "assumptions.unit_size_targets_sf.{} must satisfy min <= target <= max",
                    t.as_key()
                ),
            ));
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct ResolvedVariable {
    pub key: String,
    pub phase: SolvePhase,
    pub source: ValueSource,
    pub derived_value: Option<ScalarValue>,
    pub override_value: Option<ScalarValue>,
    pub resolved_value: ScalarValue,
    pub unit: Option<String>,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct VariableBook {
    pub vars: BTreeMap<String, ResolvedVariable>,
}

impl VariableBook {
    pub fn insert(
        &mut self,
        key: &str,
        phase: SolvePhase,
        source: ValueSource,
        derived_value: Option<ScalarValue>,
        override_value: Option<ScalarValue>,
        resolved_value: ScalarValue,
        unit: Option<&str>,
        depends_on: &[&str],
    ) {
        self.vars.insert(
            key.to_string(),
            ResolvedVariable {
                key: key.to_string(),
                phase,
                source,
                derived_value,
                override_value,
                resolved_value,
                unit: unit.map(|x| x.to_string()),
                depends_on: depends_on.iter().map(|x| x.to_string()).collect(),
            },
        );
    }

    pub fn get(&self, key: &str) -> Option<&ResolvedVariable> {
        self.vars.get(key)
    }

    pub fn values(&self) -> Vec<ResolvedVariable> {
        self.vars.values().cloned().collect()
    }
}

pub fn phase_for_override_key(key: &str) -> SolvePhase {
    if key.starts_with("jurisdiction_profile")
        || key.starts_with("building_shape")
        || key.starts_with("building_construction_type")
        || key.starts_with("site_polygon")
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

/* =============================== assumptions ============================== */

#[derive(Debug, Clone)]
pub struct GeometryAssumption {
    pub site_inset_ft: f64,
    pub template_snap_ft: f64,
    pub polygon_snap_ft: f64,
    pub daylight_depth_cap_ft: f64,
    pub unit_rect_min_width_ft: f64,
    pub unit_rect_min_depth_ft: f64,
    pub min_courtyard_width_ft: f64,
    pub min_wing_width_ft: f64,
    pub wall_loss_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct EconomicAssumption {
    pub revenue_years: f64,
    pub vacancy_months_per_year: f64,
    pub efficiency_weight: f64,
    pub rent_per_sf_studio: f64,
    pub rent_per_sf_one_bed: f64,
    pub rent_per_sf_two_bed: f64,
    pub rent_per_sf_three_bed: f64,
    pub cost_per_sf_studio: f64,
    pub cost_per_sf_one_bed: f64,
    pub cost_per_sf_two_bed: f64,
    pub cost_per_sf_three_bed: f64,
}

#[derive(Debug, Clone)]
pub struct CorridorCoreAssumption {
    pub preferred_residential_corridor_ft: f64,
    pub internal_corridor_ft: f64,
    pub single_loaded_corridor_ft: f64,
    pub perimeter_corridor_ft: f64,
    pub elevator_lobby_width_min_ft: f64,
    pub elevator_lobby_length_min_ft: f64,
    pub entry_lobby_sf_per_unit_low_mid: f64,
    pub entry_lobby_sf_per_unit_high_tower: f64,
    pub entry_lobby_interp_units_1: f64,
    pub entry_lobby_interp_sf_1: f64,
    pub entry_lobby_interp_units_2: f64,
    pub entry_lobby_interp_sf_2: f64,
    pub entry_wind_lobby_disable_for_affordable_profile: bool,
    pub entry_wind_lobby_enable_min_stories_exclusive: u32,
    pub entry_wind_lobby_enable_min_units_exclusive: u32,
    pub entry_wind_lobby_units_1: f64,
    pub entry_wind_lobby_qty_1: f64,
    pub entry_wind_lobby_units_2: f64,
    pub entry_wind_lobby_qty_2: f64,
    pub entry_wind_lobby_room_w_ft: f64,
    pub entry_wind_lobby_room_d_ft: f64,
}

#[derive(Debug, Clone)]
pub struct VerticalTransportAssumption {
    pub low_rise_max_stories: u32,
    pub mid_rise_max_stories: u32,
    pub occupant_load_factor_residential_sf_per_occ: f64,
    pub operations_staff_per_units: f64,
    pub persons_weight_lb: f64,
    pub passenger_rated_load_lb: f64,
    pub passenger_machine_room_on_roof: bool,
    pub passenger_machine_room_electric_guide: Vec<ElevatorMachineRoomGuidePoint>,
    pub passenger_machine_room_hydraulic_guide: Vec<ElevatorMachineRoomGuidePoint>,
    pub freight_machine_room_width_ft: f64,
    pub freight_machine_room_depth_ft: f64,
    pub wheelchair_lift_enabled: bool,
    pub wheelchair_lift_stop_count: u32,
    pub wheelchair_lift_landing_ewa_sf: f64,
    pub wheelchair_lift_landing_code_min_sf: f64,
    pub wheelchair_lift_units_1: f64,
    pub wheelchair_lift_qty_1: f64,
    pub wheelchair_lift_units_2: f64,
    pub wheelchair_lift_qty_2: f64,
    pub passenger_max_loading_ratio: f64,
    pub passenger_speed_ft_per_min: f64,
    pub passenger_units_per_car_common: f64,
    pub handling_target_low: f64,
    pub handling_target_mid: f64,
    pub handling_target_high: f64,
    pub interval_low_s: f64,
    pub interval_mid_s: f64,
    pub interval_high_upto_19_s: f64,
    pub interval_high_20_plus_s: f64,
    pub freight_zero_to_150: u32,
    pub freight_up_to_300: u32,
    pub freight_over_300: u32,
    pub stair_riser_in: f64,
    pub stair_tread_in: f64,
    pub stair_width_min_in: f64,
    pub stair_width_per_occ_in: f64,
    pub stair_additional_default: u32,
}

#[derive(Debug, Clone)]
pub struct SupportAssumption {
    pub bicycle_rack_length_ft: f64,
    pub bicycle_rack_width_ft: f64,
    pub bicycle_rack_aisle_ft: f64,
    pub bicycle_repair_area_sf: f64,
    pub bicycle_repair_area_long_term_stall_threshold: u32,
    pub bicycle_repair_units_1: f64,
    pub bicycle_repair_qty_1: f64,
    pub bicycle_repair_units_2: f64,
    pub bicycle_repair_qty_2: f64,
    pub common_laundry_pair_w_ft: f64,
    pub common_laundry_pair_d_ft: f64,
    pub common_laundry_pair_clearance_sf: f64,
    pub common_laundry_aux_units_1: f64,
    pub common_laundry_aux_sf_1: f64,
    pub common_laundry_aux_units_2: f64,
    pub common_laundry_aux_sf_2: f64,
    pub common_laundry_room_min_sf: f64,
    pub mailbox_per_unit: f64,
    pub locker_per_mailbox_ratio: f64,
    pub mailboxes_per_cabinet: f64,
    pub lockers_per_cabinet: f64,
    pub mail_cabinet_width_in: f64,
    pub mail_cabinet_depth_in: f64,
    pub mail_front_clear_depth_in: f64,
    pub mail_room_sf_per_du_min: f64,
    pub mail_room_sf_per_du_max: f64,
    pub general_storage_units_1: f64,
    pub general_storage_qty_1: f64,
    pub general_storage_units_2: f64,
    pub general_storage_qty_2: f64,
    pub general_storage_room_w_ft: f64,
    pub general_storage_room_d_ft: f64,
    pub janitor_units_1: f64,
    pub janitor_qty_1: f64,
    pub janitor_units_2: f64,
    pub janitor_qty_2: f64,
    pub janitor_room_w_ft: f64,
    pub janitor_room_d_ft: f64,
    pub parcel_storage_units_1: f64,
    pub parcel_storage_qty_1: f64,
    pub parcel_storage_units_2: f64,
    pub parcel_storage_qty_2: f64,
    pub parcel_storage_room_w_ft: f64,
    pub parcel_storage_room_d_ft: f64,
    pub cold_storage_units_1: f64,
    pub cold_storage_qty_1: f64,
    pub cold_storage_units_2: f64,
    pub cold_storage_qty_2: f64,
    pub cold_storage_room_w_ft: f64,
    pub cold_storage_room_d_ft: f64,
    pub leasing_office_units_1: f64,
    pub leasing_office_qty_1: f64,
    pub leasing_office_units_2: f64,
    pub leasing_office_qty_2: f64,
    pub leasing_office_room_w_ft: f64,
    pub leasing_office_room_d_ft: f64,
    pub manager_office_affordable_only: bool,
    pub manager_office_units_1: f64,
    pub manager_office_qty_1: f64,
    pub manager_office_units_2: f64,
    pub manager_office_qty_2: f64,
    pub manager_office_room_w_ft: f64,
    pub manager_office_room_d_ft: f64,
    pub cctv_room_enabled: bool,
    pub cctv_room_min_sf: f64,
    pub staff_break_room_enabled: bool,
    pub staff_break_room_units_1: f64,
    pub staff_break_room_qty_1: f64,
    pub staff_break_room_units_2: f64,
    pub staff_break_room_qty_2: f64,
    pub staff_break_room_area_sf: f64,
    pub staff_locker_showers_enabled: bool,
    pub staff_locker_showers_base_sf: f64,
    pub staff_locker_showers_circulation_ratio: f64,
    pub staff_restroom_enabled: bool,
    pub staff_restroom_min_sf: f64,
    pub staff_restroom_circulation_ratio: f64,
    pub staff_restroom_fixture_1_area_sf: f64,
    pub staff_restroom_fixture_2_area_sf: f64,
    pub staff_restroom_fixture_3_area_sf: f64,
    pub trash_occupants_per_studio: f64,
    pub trash_occupants_per_one_bed: f64,
    pub trash_occupants_per_two_bed: f64,
    pub trash_occupants_per_three_bed: f64,
    pub trash_volume_cy_per_person_per_week: f64,
    pub trash_pickups_per_week: f64,
    pub trash_room_max_distance_ft: f64,
    pub trash_dumpster_length_ft: f64,
    pub trash_dumpster_fill_factor: f64,
    pub trash_clearance_factor: f64,
    pub trash_recycling_ratio: f64,
    pub trash_compost_ratio: f64,
    pub trash_recycling_room_enabled: bool,
    pub trash_compost_room_enabled: bool,
    pub trash_chute_room_area_sf: f64,
    pub trash_chute_min_total_stories: u32,
    pub trash_compactor_width_ft: f64,
    pub trash_compactor_length_ft: f64,
    pub trash_compactor_side_clear_ft: f64,
    pub trash_compactor_front_clear_ft: f64,
    pub trash_compaction_ratio: f64,
    pub recycling_units_small_max: u32,
    pub recycling_area_small_sf: f64,
    pub recycling_units_medium_max: u32,
    pub recycling_area_medium_sf: f64,
    pub recycling_area_large_sf: f64,
    pub trash_vestibule_with_chute_sf: f64,
    pub trash_vestibule_without_chute_sf: f64,
    pub trash_vestibule_story_enable_min: u32,
    pub trash_vestibule_qty_per_res_floor: f64,
    pub parking_control_piece_1_max_stalls: u32,
    pub parking_control_enable_min_stalls: u32,
    pub parking_control_piece_1_slope: f64,
    pub parking_control_piece_1_intercept_sf: f64,
    pub parking_control_piece_2_max_stalls: u32,
    pub parking_control_piece_2_slope: f64,
    pub parking_control_piece_2_min_sf: f64,
    pub parking_control_piece_3_slope: f64,
    pub parking_control_piece_3_intercept_sf: f64,
    pub loading_dock_units_per_bay: f64,
    pub loading_dock_default_sf: f64,
    pub loading_zone_default_sf: f64,
}

#[derive(Debug, Clone)]
pub struct AmenityCatalogEntry {
    pub name: String,
    pub area_sf: f64,
    pub indoor: bool,
}

#[derive(Debug, Clone)]
pub struct AmenityAssumption {
    pub indoor_min_sf_per_du: f64,
    pub outdoor_min_sf_per_du: f64,
    pub multiplier_min_code: f64,
    pub multiplier_balanced: f64,
    pub multiplier_premium: f64,
    pub multiplier_user_selected: f64,
    pub resident_restroom_min_sf: f64,
    pub resident_restroom_circulation_ratio: f64,
    pub resident_restroom_wc_area_sf: f64,
    pub resident_restroom_urinal_area_sf: f64,
    pub resident_restroom_lavatory_area_sf: f64,
    pub amenity_storage_ratio: f64,
    pub outdoor_amenity_circulation_ratio: f64,
    pub catalog: Vec<AmenityCatalogEntry>,
}

#[derive(Debug, Clone)]
pub struct ParkingAssumption {
    pub stalls_per_studio: f64,
    pub stalls_per_one_bed: f64,
    pub stalls_per_two_bed: f64,
    pub stalls_per_three_bed: f64,
    pub retail_sf_per_stall: f64,
    pub gross_sf_per_stall_surface: f64,
    pub gross_sf_per_stall_podium: f64,
    pub gross_sf_per_stall_structured: f64,
    pub gross_sf_per_stall_underground: f64,
    pub gross_sf_per_stall_mixed: f64,
}

#[derive(Debug, Clone)]
pub struct ElectricalGeneratorOption {
    pub standby_rating_kva: f64,
    pub installed_cost_usd: f64,
    pub added_clearance_footprint_sf: f64,
}

#[derive(Debug, Clone)]
pub struct ElectricalAmpSizedEquipmentOption {
    pub max_amps: f64,
    pub width_in: f64,
    pub depth_in: f64,
}

#[derive(Debug, Clone)]
pub struct KvaSizedAreaOption {
    pub max_kva: f64,
    pub area_sf: f64,
}

#[derive(Debug, Clone)]
pub struct ElectricalTransformerSelectionOption {
    pub rating_kva: f64,
    pub selection_cost_usd: f64,
}

#[derive(Debug, Clone)]
pub struct RuleOfThumbRoomGuidePoint {
    pub load_kw: f64,
    pub width_ft: f64,
    pub depth_ft: f64,
}

#[derive(Debug, Clone)]
pub struct ElevatorMachineRoomGuidePoint {
    pub max_rated_load_lb: f64,
    pub width_ft: f64,
    pub depth_ft: f64,
}

#[derive(Debug, Clone)]
pub struct DiameterSizedEquipmentOption {
    pub max_diameter_in: f64,
    pub width_in: f64,
    pub depth_in: f64,
}

#[derive(Debug, Clone)]
pub struct BohAssumption {
    pub mpoe_units_1: f64,
    pub mpoe_qty_1: f64,
    pub mpoe_units_2: f64,
    pub mpoe_qty_2: f64,
    pub mpoe_room_w_ft: f64,
    pub mpoe_room_d_ft: f64,
    pub idf_units_1: f64,
    pub idf_qty_1: f64,
    pub idf_units_2: f64,
    pub idf_qty_2: f64,
    pub idf_room_w_ft: f64,
    pub idf_room_d_ft: f64,
    pub idf_room_max_sf: f64,
    pub idf_enable_min_stories: u32,
    pub das_a: f64,
    pub das_b: f64,
    pub water_filtration_enabled: bool,
    pub building_occupants_per_studio: f64,
    pub building_occupants_per_one_bedroom: f64,
    pub building_occupants_per_two_bedroom: f64,
    pub building_occupants_per_three_bedroom: f64,
    pub water_filtration_a: f64,
    pub water_filtration_b: f64,
    pub grease_interceptor_room_enabled: bool,
    pub grease_interceptor_tank_size_gal: f64,
    pub grease_interceptor_a: f64,
    pub grease_interceptor_b: f64,
    pub rainwater_enabled: bool,
    pub rainwater_sum_width_ft: f64,
    pub rainwater_sum_depth_ft: f64,
    pub rainwater_a: f64,
    pub rainwater_b: f64,
    pub rainwater_c: f64,
    pub plumbing_riser_enabled: bool,
    pub plumbing_riser_units_a: f64,
    pub plumbing_riser_stories_b: f64,
    pub plumbing_riser_c: f64,
    pub water_prv_closet_enabled: bool,
    pub water_prv_closet_enable_min_above_grade_stories: u32,
    pub water_prv_closet_story_a: f64,
    pub water_prv_closet_b: f64,
    pub fire_pump_room_enable_min_total_stories: u32,
    pub fire_pump_room_design_fire_flow_gpm: f64,
    pub fire_pump_room_jockey_controller_width_ft: f64,
    pub fire_pump_room_jockey_controller_depth_ft: f64,
    pub fire_pump_room_diesel_fuel_tank_enabled: bool,
    pub fire_pump_room_diesel_fuel_tank_plan_factor: f64,
    pub fire_pump_room_pump_length_ft: f64,
    pub fire_pump_room_pump_width_ft: f64,
    pub fire_pump_room_front_clear_ft: f64,
    pub fire_pump_room_side_back_clear_ft: f64,
    pub fire_pump_room_min_sf: f64,
    pub fire_pump_room_max_sf: f64,
    pub fire_control_min_sf: f64,
    pub fire_control_equipment_rack_count: u32,
    pub sprinkler_riser_enable_max_stories_exclusive: u32,
    pub sprinkler_riser_default_sf: f64,
    pub electrical_customer_station_indoor_enabled: bool,
    pub electrical_customer_station_indoor_transformer_base_length_ft: f64,
    pub electrical_customer_station_indoor_transformer_length_scale_ft: f64,
    pub electrical_customer_station_indoor_transformer_base_width_ft: f64,
    pub electrical_customer_station_indoor_transformer_width_scale_ft: f64,
    pub electrical_customer_station_indoor_transformer_reference_kva: f64,
    pub electrical_customer_station_indoor_transformer_front_clear_ft: f64,
    pub electrical_customer_station_indoor_transformer_rear_clear_ft: f64,
    pub electrical_customer_station_indoor_transformer_side_clear_ft: f64,
    pub electrical_customer_station_indoor_transformer_wall_buffer_ft: f64,
    pub electrical_customer_station_indoor_gear_envelope_length_ft: f64,
    pub electrical_customer_station_indoor_gear_envelope_width_ft: f64,
    pub electrical_customer_station_indoor_service_aisle_ft: f64,
    pub electrical_customer_station_indoor_circulation_ratio: f64,
    pub electrical_customer_station_indoor_ancillary_fixed_per_floor_sf: f64,
    pub electrical_customer_station_indoor_transformer_vault_floors: u32,
    pub electrical_utility_infrastructure_exterior_load_growth_ratio: f64,
    pub electrical_utility_infrastructure_exterior_layout_growth_ratio: f64,
    pub electrical_utility_infrastructure_exterior_other_area_sf: f64,
    pub electrical_utility_infrastructure_exterior_transformer_options:
        Vec<ElectricalTransformerSelectionOption>,
    pub electrical_utility_infrastructure_exterior_dry_area_guide: Vec<KvaSizedAreaOption>,
    pub electrical_utility_infrastructure_exterior_pad_area_guide: Vec<KvaSizedAreaOption>,
    pub electrical_service_input_voltage_v: f64,
    pub electrical_generator_output_voltage_v: f64,
    pub electrical_generator_room_enable_min_stories: u32,
    pub electrical_generator_site_factor: f64,
    pub electrical_generator_growth_ratio: f64,
    pub electrical_generator_fire_alarm_kva: f64,
    pub electrical_generator_smoke_control_kw: f64,
    pub electrical_generator_smoke_control_power_factor: f64,
    pub electrical_generator_smoke_control_demand_factor: f64,
    pub electrical_generator_domestic_booster_kw: f64,
    pub electrical_generator_domestic_booster_power_factor: f64,
    pub electrical_generator_domestic_booster_demand_factor: f64,
    pub electrical_generator_fire_pump_start_kva: f64,
    pub electrical_generator_emergency_start_kva: f64,
    pub electrical_generator_mandatory_start_kva: f64,
    pub electrical_generator_optional_start_kva: f64,
    pub electrical_generator_options: Vec<ElectricalGeneratorOption>,
    pub electrical_ats_ev_qty: u32,
    pub electrical_ats_critical_qty: u32,
    pub electrical_ats_front_clear_ft: f64,
    pub electrical_ats_two_side_clear_ft: f64,
    pub electrical_ats_two_equipment_clear_ft: f64,
    pub electrical_ats_growth_ratio: f64,
    pub electrical_ats_step_down_small_max_kva: f64,
    pub electrical_ats_step_down_small_width_in: f64,
    pub electrical_ats_step_down_small_depth_in: f64,
    pub electrical_ats_step_down_large_width_in: f64,
    pub electrical_ats_step_down_large_depth_in: f64,
    pub electrical_ats_generator_distribution_width_in: f64,
    pub electrical_ats_generator_distribution_depth_in: f64,
    pub electrical_ats_service_entrance_breaker_width_in: f64,
    pub electrical_ats_service_entrance_breaker_depth_in: f64,
    pub electrical_ats_equipment_sizing: Vec<ElectricalAmpSizedEquipmentOption>,
    pub electrical_elir_enable_min_units: u32,
    pub electrical_elir_lighting_power_factor: f64,
    pub electrical_elir_room_guide: Vec<RuleOfThumbRoomGuidePoint>,
    pub electrical_ups_room_enable_min_units: u32,
    pub electrical_ups_room_low_rise_max_stories: u32,
    pub electrical_ups_room_low_rise_min_available_roof_ratio: f64,
    pub electrical_ups_room_backup_time_hr: f64,
    pub electrical_ups_room_capacity_factor: f64,
    pub electrical_ups_room_battery_cabinet_kwh: f64,
    pub electrical_ups_room_battery_dod: f64,
    pub electrical_ups_room_battery_age_factor: f64,
    pub electrical_ups_room_power_cabinet_kwh: f64,
    pub electrical_ups_room_power_cabinet_qty_offset: u32,
    pub electrical_ups_room_pcs_kwh: f64,
    pub electrical_ups_room_distribution_cabinet_kwh: f64,
    pub electrical_ups_room_battery_width_in: f64,
    pub electrical_ups_room_battery_depth_in: f64,
    pub electrical_ups_room_power_width_in: f64,
    pub electrical_ups_room_power_depth_in: f64,
    pub electrical_ups_room_pcs_width_in: f64,
    pub electrical_ups_room_pcs_depth_in: f64,
    pub electrical_ups_room_distribution_width_in: f64,
    pub electrical_ups_room_distribution_depth_in: f64,
    pub electrical_ups_room_front_clear_ft: f64,
    pub electrical_ups_room_two_side_clear_ft: f64,
    pub electrical_ups_room_between_equipment_clear_ft: f64,
    pub electrical_ups_room_hvac_egress_ratio: f64,
    pub mechanical_ahu_room_enable_min_supply_air_cfm: f64,
    pub mechanical_ahu_room_use_cono_sensor_control: bool,
    pub mechanical_ahu_room_residential_air_cfm_per_sf: f64,
    pub mechanical_ahu_room_residential_air_cfm_per_bedroom: f64,
    pub mechanical_ahu_room_residential_bedroom_offset: f64,
    pub mechanical_ahu_room_non_residential_air_cfm_per_sf: f64,
    pub mechanical_ahu_room_parking_air_cfm_per_sf_full_on: f64,
    pub mechanical_ahu_room_parking_air_cfm_per_sf_sensor_control: f64,
    pub mechanical_ahu_room_width_base_ft: f64,
    pub mechanical_ahu_room_width_per_10k_cfm_ft: f64,
    pub mechanical_ahu_room_equipment_depth_base_ft: f64,
    pub mechanical_ahu_room_equipment_depth_per_sqrt_1k_cfm_ft: f64,
    pub mechanical_ahu_room_front_clear_ft: f64,
    pub mechanical_ahu_room_side_clear_ft: f64,
    pub mechanical_ventilation_riser_enable_min_supply_air_cfm: f64,
    pub mechanical_ventilation_riser_small_units_max: u32,
    pub mechanical_ventilation_riser_small_density_sf_per_du: f64,
    pub mechanical_ventilation_riser_small_density_max_ratio: f64,
    pub mechanical_ventilation_riser_small_module_tons: f64,
    pub mechanical_ventilation_riser_small_module_qty_divisor: f64,
    pub mechanical_ventilation_riser_small_module_area_sf: f64,
    pub mechanical_ventilation_riser_large_module_coverage_sf: f64,
    pub mechanical_ventilation_riser_large_module_area_sf: f64,
    pub mechanical_ventilation_riser_residential_air_cfm_per_sf: f64,
    pub mechanical_ventilation_riser_residential_air_cfm_per_bedroom: f64,
    pub mechanical_ventilation_riser_residential_bedroom_offset: f64,
    pub mechanical_ventilation_riser_duct_velocity_fpm: f64,
    pub mechanical_ventilation_riser_pipe_diameter_in: f64,
    pub mechanical_ventilation_riser_clearance_in: f64,
    pub mechanical_ventilation_riser_bathroom_exhaust_cfm: f64,
    pub mechanical_ventilation_riser_unit_kitchen_exhaust_cfm: f64,
    pub mechanical_ventilation_riser_bathrooms_per_studio: u32,
    pub mechanical_ventilation_riser_bathrooms_per_one_bedroom: u32,
    pub mechanical_ventilation_riser_bathrooms_per_two_bedroom: u32,
    pub mechanical_ventilation_riser_bathrooms_per_three_bedroom: u32,
    pub mechanical_pad_outdoor_enabled: bool,
    pub mechanical_pad_outdoor_split_max_units: u32,
    pub mechanical_pad_outdoor_split_roof_sf_per_unit: f64,
    pub mechanical_pad_outdoor_split_max_roof_coverage_ratio: f64,
    pub mechanical_pad_outdoor_split_width_ft: f64,
    pub mechanical_pad_outdoor_split_depth_ft: f64,
    pub mechanical_pad_outdoor_vrf_tons: f64,
    pub mechanical_pad_outdoor_vrf_width_ft: f64,
    pub mechanical_pad_outdoor_vrf_depth_ft: f64,
    pub mechanical_pad_outdoor_residential_sf_per_ton: f64,
    pub mechanical_pad_outdoor_layout_divisor: f64,
    pub mechanical_pad_outdoor_service_aisle_ft: f64,
    pub mechanical_pad_outdoor_equipment_clear_ft: f64,
    pub mechanical_pad_outdoor_front_clear_ft: f64,
    pub mechanical_pad_outdoor_side_clear_ft: f64,
    pub commercial_kitchen_shaft_range_qty_units_1: f64,
    pub commercial_kitchen_shaft_range_qty_1: f64,
    pub commercial_kitchen_shaft_range_qty_units_2: f64,
    pub commercial_kitchen_shaft_range_qty_2: f64,
    pub commercial_kitchen_shaft_range_width_ft: f64,
    pub commercial_kitchen_shaft_hood_length_offset_ft: f64,
    pub commercial_kitchen_shaft_use_hood_ul710: bool,
    pub commercial_kitchen_shaft_range_type_electric: bool,
    pub commercial_kitchen_shaft_ul710_exhaust_rate_cfm_per_ft: f64,
    pub commercial_kitchen_shaft_electric_exhaust_rate_cfm_per_ft: f64,
    pub commercial_kitchen_shaft_gas_exhaust_rate_cfm_per_ft: f64,
    pub commercial_kitchen_shaft_diversity_factor: f64,
    pub commercial_kitchen_shaft_future_expansion_ratio: f64,
    pub commercial_kitchen_shaft_exhaust_velocity_fpm: f64,
    pub commercial_kitchen_shaft_round_duct_upsize_in: f64,
    pub commercial_kitchen_shaft_clearance_in: f64,
    pub commercial_kitchen_shaft_waste_vent_width_in: f64,
    pub commercial_kitchen_shaft_other_system_area_sf: f64,
    pub domestic_water_booster_room_enable_min_stories: u32,
    pub domestic_water_booster_room_story_height_ft: f64,
    pub domestic_water_booster_room_future_growth_ratio: f64,
    pub domestic_water_booster_room_continuous_demand_gpm: f64,
    pub domestic_water_booster_room_residential_wsfu_per_studio: f64,
    pub domestic_water_booster_room_residential_wsfu_per_one_bedroom: f64,
    pub domestic_water_booster_room_residential_wsfu_per_two_bedroom: f64,
    pub domestic_water_booster_room_residential_wsfu_per_three_bedroom: f64,
    pub domestic_water_booster_room_public_wc_wsfu: f64,
    pub domestic_water_booster_room_public_urinal_wsfu: f64,
    pub domestic_water_booster_room_public_lav_wsfu: f64,
    pub domestic_water_booster_room_public_kitchen_sink_qty: u32,
    pub domestic_water_booster_room_public_kitchen_sink_wsfu: f64,
    pub domestic_water_booster_room_public_service_sink_qty: u32,
    pub domestic_water_booster_room_public_service_sink_wsfu: f64,
    pub domestic_water_booster_room_peak_flow_cubic_a: f64,
    pub domestic_water_booster_room_peak_flow_cubic_b: f64,
    pub domestic_water_booster_room_peak_flow_cubic_c: f64,
    pub domestic_water_booster_room_peak_flow_cubic_d: f64,
    pub domestic_water_booster_room_pipe_velocity_fps: f64,
    pub domestic_water_booster_room_pipe_capacity_constant: f64,
    pub domestic_water_booster_room_hazen_williams_c: f64,
    pub domestic_water_booster_room_fitting_friction_loss_factor: f64,
    pub domestic_water_booster_room_friction_loss_max_psi_per_100ft: f64,
    pub domestic_water_booster_room_residual_pressure_psi: f64,
    pub domestic_water_booster_room_meter_backflow_prv_valves_psi: f64,
    pub domestic_water_booster_room_safety_margin_psi: f64,
    pub domestic_water_booster_room_lowest_city_pressure_psi: f64,
    pub domestic_water_booster_room_run_fraction: f64,
    pub domestic_water_booster_room_high_story_three_pump_min_stories: u32,
    pub domestic_water_booster_room_duty_pump_low_flow_max_gpm: f64,
    pub domestic_water_booster_room_duty_pump_mid_flow_max_gpm: f64,
    pub domestic_water_booster_room_low_story_max_for_longer_run: u32,
    pub domestic_water_booster_room_min_run_time_low_story_min: f64,
    pub domestic_water_booster_room_min_run_time_high_story_min: f64,
    pub domestic_water_booster_room_expansion_tank_delta_p_psi: f64,
    pub domestic_water_booster_room_expansion_tank_capacity_gal: f64,
    pub domestic_water_booster_room_expansion_tank_diameter_curve_a: f64,
    pub domestic_water_booster_room_expansion_tank_diameter_curve_b: f64,
    pub domestic_water_booster_room_expansion_tank_diameter_curve_c: f64,
    pub domestic_water_booster_room_standby_pump_qty: u32,
    pub domestic_water_booster_room_pump_width_in: f64,
    pub domestic_water_booster_room_pump_depth_in: f64,
    pub domestic_water_booster_room_control_panel_qty: u32,
    pub domestic_water_booster_room_control_panel_width_in: f64,
    pub domestic_water_booster_room_control_panel_depth_in: f64,
    pub domestic_water_booster_room_front_clear_ft: f64,
    pub domestic_water_booster_room_side_clear_ft: f64,
    pub domestic_water_booster_room_equipment_clear_ft: f64,
    pub cistern_water_storage_room_enabled: bool,
    pub cistern_water_storage_room_a: f64,
    pub cistern_water_storage_room_b: f64,
    pub backflow_preventer_room_enabled: bool,
    pub backflow_preventer_max_pipe_diameter_in: f64,
    pub backflow_preventer_pipe_velocity_fps: f64,
    pub backflow_preventer_pipe_capacity_constant: f64,
    pub backflow_preventer_domestic_backflow_guide: Vec<DiameterSizedEquipmentOption>,
    pub backflow_preventer_fire_backflow_guide: Vec<DiameterSizedEquipmentOption>,
    pub backflow_preventer_fire_backflow_qty: u32,
    pub backflow_preventer_irrigation_qty: u32,
    pub backflow_preventer_irrigation_width_in: f64,
    pub backflow_preventer_irrigation_depth_in: f64,
    pub backflow_preventer_front_clear_ft: f64,
    pub backflow_preventer_side_clear_ft: f64,
    pub backflow_preventer_equipment_clear_ft: f64,
    pub central_water_heating_room_indoor_enabled: bool,
    pub central_water_heating_room_sum_width_ft: f64,
    pub central_water_heating_room_sum_depth_ft: f64,
    pub central_water_heating_room_a: f64,
    pub central_water_heating_room_b: f64,
    pub central_water_heating_room_c: f64,
    pub central_water_heating_pad_outdoor_enabled: bool,
    pub central_water_heating_pad_outdoor_sum_width_ft: f64,
    pub central_water_heating_pad_outdoor_sum_depth_ft: f64,
    pub central_water_heating_pad_outdoor_a: f64,
    pub central_water_heating_pad_outdoor_b: f64,
    pub central_water_heating_pad_outdoor_c: f64,
    pub graywater_system_room_enabled: bool,
    pub graywater_system_room_sum_width_ft: f64,
    pub graywater_system_room_sum_depth_ft: f64,
    pub graywater_system_room_a: f64,
    pub graywater_system_room_b: f64,
    pub graywater_system_room_c: f64,
    pub gas_utility_room_enabled: bool,
    pub gas_utility_master_meter_count: u32,
    pub gas_utility_single_meter_length_ft: f64,
    pub gas_utility_per_meter_length_ft: f64,
    pub gas_utility_length_offset_ft: f64,
    pub gas_utility_room_width_ft: f64,
    pub gas_utility_dcu_closet_sf: f64,
    pub gas_meter_space_alcove_enabled: bool,
    pub gas_meter_space_alcove_bank_depth_ft: f64,
    pub gas_meter_space_alcove_front_clear_ft: f64,
    pub gas_meter_space_alcove_single_bank_width_ft: f64,
    pub gas_meter_space_alcove_two_meter_bank_width_ft: f64,
    pub gas_meter_space_alcove_additional_meter_width_ft: f64,
    pub gas_meter_space_alcove_side_clear_ft: f64,
    pub sub_electrical_enable_min_units: u32,
    pub sub_electrical_enable_min_gba_sf: f64,
    pub sub_electrical_units_per_panelboard: f64,
    pub sub_electrical_panel_depth_ft: f64,
    pub sub_electrical_front_clear_ft: f64,
    pub sub_electrical_wiggle_ft: f64,
    pub sub_electrical_panel_width_ft: f64,
    pub sub_electrical_side_clear_ft: f64,
    pub sub_electrical_growth_ratio: f64,
    pub sub_electrical_option_1_max_units_per_floor: f64,
    pub sub_electrical_option_2_max_units_per_floor: f64,
    pub sub_electrical_option_2_w_ft: f64,
    pub sub_electrical_option_2_d_ft: f64,
    pub sub_electrical_option_3_w_ft: f64,
    pub sub_electrical_option_3_d_ft: f64,
    pub electrical_main_room_enable_min_units: u32,
    pub electrical_elevator_kva_per_car: f64,
    pub electrical_motor_constant_kva: f64,
    pub electrical_corridor_w_per_sf: f64,
    pub electrical_retail_w_per_sf: f64,
    pub electrical_shaft_stair_w_per_sf: f64,
    pub electrical_entry_lobby_w_per_sf: f64,
    pub electrical_indoor_parking_w_per_sf: f64,
    pub electrical_residential_lighting_w_per_sf: f64,
    pub electrical_dwelling_unit_appliance_connected_kva_per_du: f64,
    pub electrical_dwelling_unit_misc_connected_kva_per_du: f64,
    pub electrical_stair_shaft_area_sf_per_stair_per_floor: f64,
}

#[derive(Debug, Clone)]
pub struct SolverAssumption {
    pub unit_size_quantile_max_far: f64,
    pub unit_size_quantile_max_du: f64,
    pub unit_size_quantile_balanced_yield: f64,
    pub repeatability_weight_default: f64,
}

#[derive(Debug, Clone)]
pub struct PreliminaryAreaRatioFormula {
    pub slope_inv_stories: f64,
    pub slope_inv_gfa: f64,
    pub slope_story_over_gfa: f64,
    pub intercept: f64,
}

#[derive(Debug, Clone)]
pub struct PreliminaryAreaAssumption {
    pub default_affordable_support_profile: bool,
    pub support_staff_ratio_affordable: PreliminaryAreaRatioFormula,
    pub support_staff_ratio_market: PreliminaryAreaRatioFormula,
    pub coarse_corridor_gfa_break_sf: f64,
    pub coarse_corridor_width_low_ft: f64,
    pub coarse_corridor_width_high_ft: f64,
    pub coarse_corridor_loading_factor: f64,
    pub fixed_common_area_sf: f64,
    pub stair_vestibule_area_sf: f64,
    pub stair_vestibule_count_per_floor: f64,
    pub stair_occ_normalizer_sf: f64,
    pub elevator_lobby_per_bank_low_mid_sf: f64,
    pub elevator_lobby_per_bank_high_sf: f64,
    pub elevator_lobby_split_factor: f64,
    pub elevator_lobby_normalizer_sf: f64,
    pub open_space_weight: f64,
    pub open_space_sf_studio: f64,
    pub open_space_sf_one_bed: f64,
    pub open_space_sf_two_bed: f64,
    pub open_space_sf_three_bed: f64,
    pub circulation_small_gba_threshold_sf: f64,
    pub circulation_large_gba_threshold_sf: f64,
    pub circulation_small_ratio: f64,
    pub circulation_large_ratio: f64,
    pub circulation_small_quad_a: f64,
    pub circulation_small_quad_b: f64,
    pub circulation_small_quad_c: f64,
    pub circulation_small_min_sf: f64,
    pub circulation_small_max_sf: f64,
    pub circulation_large_quad_a: f64,
    pub circulation_large_quad_b: f64,
    pub circulation_large_quad_c: f64,
    pub circulation_large_min_sf: f64,
    pub circulation_large_max_sf: f64,
}

#[derive(Debug, Clone, Default)]
pub struct PreliminaryAreaBudget {
    pub avg_unit_area_sf: f64,
    pub support_staff_ratio: f64,
    pub support_staff_area_sf: f64,
    pub corridor_width_ft: f64,
    pub corridor_term: f64,
    pub elevator_lobby_term: f64,
    pub stair_term: f64,
    pub open_space_term: f64,
    pub fixed_common_area_sf: f64,
    pub stair_vestibule_total_sf: f64,
    pub circulation_area_sf: f64,
    pub residential_area_sf: f64,
    pub weighted_open_space_sf_per_unit: f64,
}

#[derive(Debug, Clone)]
pub struct AssumptionPack {
    pub geometry: GeometryAssumption,
    pub economics: EconomicAssumption,
    pub corridor_core: CorridorCoreAssumption,
    pub vertical: VerticalTransportAssumption,
    pub support: SupportAssumption,
    pub amenity: AmenityAssumption,
    pub unit_size_targets_sf: UnitSizeTargets,
    pub parking: ParkingAssumption,
    pub boh: BohAssumption,
    pub solver: SolverAssumption,
    pub preliminary_area: PreliminaryAreaAssumption,
}

impl Default for AssumptionPack {
    fn default() -> Self {
        Self {
            geometry: GeometryAssumption {
                site_inset_ft: 8.0,
                template_snap_ft: 2.0,
                polygon_snap_ft: 1.0,
                daylight_depth_cap_ft: 38.0,
                unit_rect_min_width_ft: 14.0,
                unit_rect_min_depth_ft: 18.0,
                min_courtyard_width_ft: 40.0,
                min_wing_width_ft: 28.0,
                wall_loss_ratio: 0.08,
            },
            economics: EconomicAssumption {
                revenue_years: 10.0,
                vacancy_months_per_year: 1.0,
                efficiency_weight: 0.60,
                rent_per_sf_studio: 3.45,
                rent_per_sf_one_bed: 3.35,
                rent_per_sf_two_bed: 3.20,
                rent_per_sf_three_bed: 3.10,
                cost_per_sf_studio: 330.0,
                cost_per_sf_one_bed: 325.0,
                cost_per_sf_two_bed: 320.0,
                cost_per_sf_three_bed: 318.0,
            },
            corridor_core: CorridorCoreAssumption {
                preferred_residential_corridor_ft: 6.0,
                internal_corridor_ft: 8.0,
                single_loaded_corridor_ft: 6.0,
                perimeter_corridor_ft: 7.0,
                elevator_lobby_width_min_ft: 8.0,
                elevator_lobby_length_min_ft: 11.0,
                entry_lobby_sf_per_unit_low_mid: 8.0,
                entry_lobby_sf_per_unit_high_tower: 12.0,
                entry_lobby_interp_units_1: 100.0,
                entry_lobby_interp_sf_1: 750.0,
                entry_lobby_interp_units_2: 1000.0,
                entry_lobby_interp_sf_2: 3000.0,
                entry_wind_lobby_disable_for_affordable_profile: true,
                entry_wind_lobby_enable_min_stories_exclusive: 3,
                entry_wind_lobby_enable_min_units_exclusive: 50,
                entry_wind_lobby_units_1: 100.0,
                entry_wind_lobby_qty_1: 1.0,
                entry_wind_lobby_units_2: 1000.0,
                entry_wind_lobby_qty_2: 4.0,
                entry_wind_lobby_room_w_ft: 8.0,
                entry_wind_lobby_room_d_ft: 8.0,
            },
            vertical: VerticalTransportAssumption {
                low_rise_max_stories: 4,
                mid_rise_max_stories: 9,
                occupant_load_factor_residential_sf_per_occ: 200.0,
                operations_staff_per_units: 50.0,
                persons_weight_lb: 150.0,
                passenger_rated_load_lb: 3000.0,
                passenger_machine_room_on_roof: true,
                passenger_machine_room_electric_guide: vec![
                    ElevatorMachineRoomGuidePoint {
                        max_rated_load_lb: 2000.0,
                        width_ft: 7.333333333333333,
                        depth_ft: 16.0,
                    },
                    ElevatorMachineRoomGuidePoint {
                        max_rated_load_lb: 2500.0,
                        width_ft: 8.333333333333334,
                        depth_ft: 16.0,
                    },
                    ElevatorMachineRoomGuidePoint {
                        max_rated_load_lb: 3000.0,
                        width_ft: 8.333333333333334,
                        depth_ft: 16.0,
                    },
                    ElevatorMachineRoomGuidePoint {
                        max_rated_load_lb: 3500.0,
                        width_ft: 8.333333333333334,
                        depth_ft: 16.0,
                    },
                    ElevatorMachineRoomGuidePoint {
                        max_rated_load_lb: 4000.0,
                        width_ft: 9.5,
                        depth_ft: 16.0,
                    },
                ],
                passenger_machine_room_hydraulic_guide: vec![
                    ElevatorMachineRoomGuidePoint {
                        max_rated_load_lb: 2000.0,
                        width_ft: 7.333333333333333,
                        depth_ft: 7.578947368421052,
                    },
                    ElevatorMachineRoomGuidePoint {
                        max_rated_load_lb: 2500.0,
                        width_ft: 8.333333333333334,
                        depth_ft: 7.578947368421052,
                    },
                    ElevatorMachineRoomGuidePoint {
                        max_rated_load_lb: 3000.0,
                        width_ft: 8.333333333333334,
                        depth_ft: 7.578947368421052,
                    },
                    ElevatorMachineRoomGuidePoint {
                        max_rated_load_lb: 3500.0,
                        width_ft: 8.333333333333334,
                        depth_ft: 7.578947368421052,
                    },
                    ElevatorMachineRoomGuidePoint {
                        max_rated_load_lb: 4000.0,
                        width_ft: 9.5,
                        depth_ft: 7.578947368421052,
                    },
                ],
                freight_machine_room_width_ft: 8.0,
                freight_machine_room_depth_ft: 10.0,
                wheelchair_lift_enabled: false,
                wheelchair_lift_stop_count: 2,
                wheelchair_lift_landing_ewa_sf: 20.0,
                wheelchair_lift_landing_code_min_sf: 25.0,
                wheelchair_lift_units_1: 100.0,
                wheelchair_lift_qty_1: 1.0,
                wheelchair_lift_units_2: 1000.0,
                wheelchair_lift_qty_2: 2.0,
                passenger_max_loading_ratio: 0.65,
                passenger_speed_ft_per_min: 350.0,
                passenger_units_per_car_common: 80.0,
                handling_target_low: 0.08,
                handling_target_mid: 0.07,
                handling_target_high: 0.06,
                interval_low_s: 50.0,
                interval_mid_s: 60.0,
                interval_high_upto_19_s: 50.0,
                interval_high_20_plus_s: 45.0,
                freight_zero_to_150: 0,
                freight_up_to_300: 1,
                freight_over_300: 2,
                stair_riser_in: 7.0,
                stair_tread_in: 11.0,
                stair_width_min_in: 44.0,
                stair_width_per_occ_in: 0.30,
                stair_additional_default: 1,
            },
            support: SupportAssumption {
                bicycle_rack_length_ft: 5.0,
                bicycle_rack_width_ft: 2.5,
                bicycle_rack_aisle_ft: 5.0,
                bicycle_repair_area_sf: 100.0,
                bicycle_repair_area_long_term_stall_threshold: 20,
                bicycle_repair_units_1: 100.0,
                bicycle_repair_qty_1: 1.0,
                bicycle_repair_units_2: 1000.0,
                bicycle_repair_qty_2: 2.0,
                common_laundry_pair_w_ft: 3.0,
                common_laundry_pair_d_ft: 7.0,
                common_laundry_pair_clearance_sf: 1.0,
                common_laundry_aux_units_1: 100.0,
                common_laundry_aux_sf_1: 200.0,
                common_laundry_aux_units_2: 1000.0,
                common_laundry_aux_sf_2: 500.0,
                common_laundry_room_min_sf: 100.0,
                mailbox_per_unit: 1.0,
                locker_per_mailbox_ratio: 0.20,
                mailboxes_per_cabinet: 20.0,
                lockers_per_cabinet: 2.0,
                mail_cabinet_width_in: 32.69,
                mail_cabinet_depth_in: 18.0,
                mail_front_clear_depth_in: 60.0,
                mail_room_sf_per_du_min: 1.5,
                mail_room_sf_per_du_max: 2.0,
                general_storage_units_1: 100.0,
                general_storage_qty_1: 2.0,
                general_storage_units_2: 1000.0,
                general_storage_qty_2: 10.0,
                general_storage_room_w_ft: 10.0,
                general_storage_room_d_ft: 10.0,
                janitor_units_1: 100.0,
                janitor_qty_1: 4.0,
                janitor_units_2: 1000.0,
                janitor_qty_2: 12.0,
                janitor_room_w_ft: 6.0,
                janitor_room_d_ft: 6.0,
                parcel_storage_units_1: 100.0,
                parcel_storage_qty_1: 1.0,
                parcel_storage_units_2: 1000.0,
                parcel_storage_qty_2: 8.0,
                parcel_storage_room_w_ft: 10.0,
                parcel_storage_room_d_ft: 10.0,
                cold_storage_units_1: 100.0,
                cold_storage_qty_1: 1.0,
                cold_storage_units_2: 1000.0,
                cold_storage_qty_2: 4.0,
                cold_storage_room_w_ft: 10.0,
                cold_storage_room_d_ft: 10.0,
                leasing_office_units_1: 100.0,
                leasing_office_qty_1: 1.0,
                leasing_office_units_2: 1000.0,
                leasing_office_qty_2: 2.0,
                leasing_office_room_w_ft: 10.0,
                leasing_office_room_d_ft: 10.0,
                manager_office_affordable_only: true,
                manager_office_units_1: 100.0,
                manager_office_qty_1: 1.0,
                manager_office_units_2: 1000.0,
                manager_office_qty_2: 2.0,
                manager_office_room_w_ft: 10.0,
                manager_office_room_d_ft: 10.0,
                cctv_room_enabled: true,
                cctv_room_min_sf: 60.0,
                staff_break_room_enabled: false,
                staff_break_room_units_1: 100.0,
                staff_break_room_qty_1: 0.0,
                staff_break_room_units_2: 1000.0,
                staff_break_room_qty_2: 1.0,
                staff_break_room_area_sf: 144.0,
                staff_locker_showers_enabled: true,
                staff_locker_showers_base_sf: 146.0,
                staff_locker_showers_circulation_ratio: 0.25,
                staff_restroom_enabled: true,
                staff_restroom_min_sf: 60.0,
                staff_restroom_circulation_ratio: 0.20,
                staff_restroom_fixture_1_area_sf: 25.0,
                staff_restroom_fixture_2_area_sf: 11.0,
                staff_restroom_fixture_3_area_sf: 15.0,
                trash_occupants_per_studio: 1.5,
                trash_occupants_per_one_bed: 2.0,
                trash_occupants_per_two_bed: 3.0,
                trash_occupants_per_three_bed: 4.0,
                trash_volume_cy_per_person_per_week: 0.2,
                trash_pickups_per_week: 2.0,
                trash_room_max_distance_ft: 120.0,
                trash_dumpster_length_ft: 6.0,
                trash_dumpster_fill_factor: 0.8,
                trash_clearance_factor: 2.5,
                trash_recycling_ratio: 0.3,
                trash_compost_ratio: 0.25,
                trash_recycling_room_enabled: true,
                trash_compost_room_enabled: true,
                trash_chute_room_area_sf: 30.0,
                trash_chute_min_total_stories: 10,
                trash_compactor_width_ft: 6.0,
                trash_compactor_length_ft: 12.0,
                trash_compactor_side_clear_ft: 6.0,
                trash_compactor_front_clear_ft: 4.0,
                trash_compaction_ratio: 5.0,
                recycling_units_small_max: 20,
                recycling_area_small_sf: 30.0,
                recycling_units_medium_max: 50,
                recycling_area_medium_sf: 60.0,
                recycling_area_large_sf: 100.0,
                trash_vestibule_with_chute_sf: 48.0,
                trash_vestibule_without_chute_sf: 63.0,
                trash_vestibule_story_enable_min: 2,
                trash_vestibule_qty_per_res_floor: 2.0,
                parking_control_piece_1_max_stalls: 400,
                parking_control_enable_min_stalls: 300,
                parking_control_piece_1_slope: 0.16,
                parking_control_piece_1_intercept_sf: 56.0,
                parking_control_piece_2_max_stalls: 1000,
                parking_control_piece_2_slope: 0.8,
                parking_control_piece_2_min_sf: 150.0,
                parking_control_piece_3_slope: 0.6,
                parking_control_piece_3_intercept_sf: 80.0,
                loading_dock_units_per_bay: 250.0,
                loading_dock_default_sf: 250.0,
                loading_zone_default_sf: 400.0,
            },
            amenity: AmenityAssumption {
                indoor_min_sf_per_du: 12.0,
                outdoor_min_sf_per_du: 18.0,
                multiplier_min_code: 1.00,
                multiplier_balanced: 1.35,
                multiplier_premium: 1.85,
                multiplier_user_selected: 1.20,
                resident_restroom_min_sf: 60.0,
                resident_restroom_circulation_ratio: 0.20,
                resident_restroom_wc_area_sf: 25.0,
                resident_restroom_urinal_area_sf: 11.0,
                resident_restroom_lavatory_area_sf: 15.0,
                amenity_storage_ratio: 0.08,
                outdoor_amenity_circulation_ratio: 0.2,
                catalog: vec![
                    AmenityCatalogEntry {
                        name: "fitness".to_string(),
                        area_sf: 1800.0,
                        indoor: true,
                    },
                    AmenityCatalogEntry {
                        name: "cowork".to_string(),
                        area_sf: 1400.0,
                        indoor: true,
                    },
                    AmenityCatalogEntry {
                        name: "club_room".to_string(),
                        area_sf: 2200.0,
                        indoor: true,
                    },
                    AmenityCatalogEntry {
                        name: "package_lounge".to_string(),
                        area_sf: 600.0,
                        indoor: true,
                    },
                    AmenityCatalogEntry {
                        name: "pet_spa".to_string(),
                        area_sf: 350.0,
                        indoor: true,
                    },
                    AmenityCatalogEntry {
                        name: "roof_deck".to_string(),
                        area_sf: 4000.0,
                        indoor: false,
                    },
                    AmenityCatalogEntry {
                        name: "pool".to_string(),
                        area_sf: 5000.0,
                        indoor: false,
                    },
                    AmenityCatalogEntry {
                        name: "sky_lounge".to_string(),
                        area_sf: 2500.0,
                        indoor: true,
                    },
                    AmenityCatalogEntry {
                        name: "concierge".to_string(),
                        area_sf: 481.56662406666555,
                        indoor: true,
                    },
                    AmenityCatalogEntry {
                        name: "massage".to_string(),
                        area_sf: 230.03182458013103,
                        indoor: true,
                    },
                    AmenityCatalogEntry {
                        name: "spa".to_string(),
                        area_sf: 597.09069291284686,
                        indoor: true,
                    },
                    AmenityCatalogEntry {
                        name: "sauna".to_string(),
                        area_sf: 586.03345785890519,
                        indoor: true,
                    },
                    AmenityCatalogEntry {
                        name: "theater".to_string(),
                        area_sf: 439.52509339417901,
                        indoor: true,
                    },
                ],
            },
            unit_size_targets_sf: UnitSizeTargets {
                studio: UnitSizeTarget {
                    min_sf: 450.0,
                    target_sf: 490.0,
                    max_sf: 620.0,
                },
                one_bedroom: UnitSizeTarget {
                    min_sf: 590.0,
                    target_sf: 660.0,
                    max_sf: 760.0,
                },
                two_bedroom: UnitSizeTarget {
                    min_sf: 930.0,
                    target_sf: 1010.0,
                    max_sf: 1130.0,
                },
                three_bedroom: UnitSizeTarget {
                    min_sf: 1180.0,
                    target_sf: 1260.0,
                    max_sf: 1400.0,
                },
            },
            parking: ParkingAssumption {
                stalls_per_studio: 1.0,
                stalls_per_one_bed: 1.25,
                stalls_per_two_bed: 1.5,
                stalls_per_three_bed: 2.0,
                retail_sf_per_stall: 300.0,
                gross_sf_per_stall_surface: 325.0,
                gross_sf_per_stall_podium: 360.0,
                gross_sf_per_stall_structured: 340.0,
                gross_sf_per_stall_underground: 380.0,
                gross_sf_per_stall_mixed: 355.0,
            },
            boh: BohAssumption {
                mpoe_units_1: 100.0,
                mpoe_qty_1: 1.0,
                mpoe_units_2: 1000.0,
                mpoe_qty_2: 4.0,
                mpoe_room_w_ft: 10.0,
                mpoe_room_d_ft: 10.0,
                idf_units_1: 100.0,
                idf_qty_1: 8.0,
                idf_units_2: 1000.0,
                idf_qty_2: 20.0,
                idf_room_w_ft: 4.0,
                idf_room_d_ft: 6.0,
                idf_room_max_sf: 150.0,
                idf_enable_min_stories: 3,
                das_a: 0.4758462914,
                das_b: 50.3745710039,
                water_filtration_enabled: true,
                building_occupants_per_studio: 2.0,
                building_occupants_per_one_bedroom: 2.0,
                building_occupants_per_two_bedroom: 4.0,
                building_occupants_per_three_bedroom: 6.0,
                water_filtration_a: 0.10790015829698081,
                water_filtration_b: 6.7175986302974771,
                grease_interceptor_room_enabled: true,
                grease_interceptor_tank_size_gal: 1500.0,
                grease_interceptor_a: 0.0346588243076923,
                grease_interceptor_b: 54.3273466153846,
                rainwater_enabled: true,
                rainwater_sum_width_ft: 17.0,
                rainwater_sum_depth_ft: 12.0,
                rainwater_a: 2.7837321432454201,
                rainwater_b: 6.3827406449087878,
                rainwater_c: -16.794361940379702,
                plumbing_riser_enabled: true,
                plumbing_riser_units_a: 0.40327223488749814,
                plumbing_riser_stories_b: 2.3074042039169735,
                plumbing_riser_c: -1.0502021502156955,
                water_prv_closet_enabled: true,
                water_prv_closet_enable_min_above_grade_stories: 7,
                water_prv_closet_story_a: 0.18438518503609286,
                water_prv_closet_b: 0.92764877430930426,
                fire_pump_room_enable_min_total_stories: 4,
                fire_pump_room_design_fire_flow_gpm: 1000.0,
                fire_pump_room_jockey_controller_width_ft: 3.0,
                fire_pump_room_jockey_controller_depth_ft: 3.0,
                fire_pump_room_diesel_fuel_tank_enabled: true,
                fire_pump_room_diesel_fuel_tank_plan_factor: 0.035,
                fire_pump_room_pump_length_ft: 4.8,
                fire_pump_room_pump_width_ft: 3.3,
                fire_pump_room_front_clear_ft: 3.0,
                fire_pump_room_side_back_clear_ft: 1.0,
                fire_pump_room_min_sf: 100.0,
                fire_pump_room_max_sf: 400.0,
                fire_control_min_sf: 200.0,
                fire_control_equipment_rack_count: 0,
                sprinkler_riser_enable_max_stories_exclusive: 3,
                sprinkler_riser_default_sf: 20.0,
                electrical_customer_station_indoor_enabled: true,
                electrical_customer_station_indoor_transformer_base_length_ft: 6.0,
                electrical_customer_station_indoor_transformer_length_scale_ft: 4.0,
                electrical_customer_station_indoor_transformer_base_width_ft: 4.0,
                electrical_customer_station_indoor_transformer_width_scale_ft: 2.0,
                electrical_customer_station_indoor_transformer_reference_kva: 1000.0,
                electrical_customer_station_indoor_transformer_front_clear_ft: 3.0,
                electrical_customer_station_indoor_transformer_rear_clear_ft: 3.0,
                electrical_customer_station_indoor_transformer_side_clear_ft: 3.0,
                electrical_customer_station_indoor_transformer_wall_buffer_ft: 1.0,
                electrical_customer_station_indoor_gear_envelope_length_ft: 14.0,
                electrical_customer_station_indoor_gear_envelope_width_ft: 8.0,
                electrical_customer_station_indoor_service_aisle_ft: 3.0,
                electrical_customer_station_indoor_circulation_ratio: 0.10,
                electrical_customer_station_indoor_ancillary_fixed_per_floor_sf: 0.0,
                electrical_customer_station_indoor_transformer_vault_floors: 2,
                electrical_utility_infrastructure_exterior_load_growth_ratio: 0.10,
                electrical_utility_infrastructure_exterior_layout_growth_ratio: 0.15,
                electrical_utility_infrastructure_exterior_other_area_sf: 10.0,
                electrical_utility_infrastructure_exterior_transformer_options: vec![
                    ElectricalTransformerSelectionOption {
                        rating_kva: 45.0,
                        selection_cost_usd: 14700.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 75.0,
                        selection_cost_usd: 16600.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 112.5,
                        selection_cost_usd: 18400.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 150.0,
                        selection_cost_usd: 20000.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 225.0,
                        selection_cost_usd: 22600.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 300.0,
                        selection_cost_usd: 24800.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 500.0,
                        selection_cost_usd: 29700.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 750.0,
                        selection_cost_usd: 34500.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 1000.0,
                        selection_cost_usd: 38500.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 1500.0,
                        selection_cost_usd: 45300.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 2000.0,
                        selection_cost_usd: 51000.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 2500.0,
                        selection_cost_usd: 56100.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 3000.0,
                        selection_cost_usd: 60600.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 3750.0,
                        selection_cost_usd: 66800.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 5000.0,
                        selection_cost_usd: 75900.0,
                    },
                    ElectricalTransformerSelectionOption {
                        rating_kva: 7500.0,
                        selection_cost_usd: 91100.0,
                    },
                ],
                electrical_utility_infrastructure_exterior_dry_area_guide: vec![
                    KvaSizedAreaOption {
                        max_kva: 30.0,
                        area_sf: 26.985663611111111,
                    },
                    KvaSizedAreaOption {
                        max_kva: 45.0,
                        area_sf: 26.985663611111111,
                    },
                    KvaSizedAreaOption {
                        max_kva: 75.0,
                        area_sf: 31.337499999999995,
                    },
                    KvaSizedAreaOption {
                        max_kva: 112.5,
                        area_sf: 37.141406249999996,
                    },
                    KvaSizedAreaOption {
                        max_kva: 150.0,
                        area_sf: 37.141406249999996,
                    },
                    KvaSizedAreaOption {
                        max_kva: 225.0,
                        area_sf: 40.452847222222218,
                    },
                    KvaSizedAreaOption {
                        max_kva: 300.0,
                        area_sf: 43.074447916666664,
                    },
                ],
                electrical_utility_infrastructure_exterior_pad_area_guide: vec![
                    KvaSizedAreaOption {
                        max_kva: 45.0,
                        area_sf: 115.76666666666667,
                    },
                    KvaSizedAreaOption {
                        max_kva: 75.0,
                        area_sf: 115.76666666666667,
                    },
                    KvaSizedAreaOption {
                        max_kva: 112.5,
                        area_sf: 123.43333333333334,
                    },
                    KvaSizedAreaOption {
                        max_kva: 150.0,
                        area_sf: 123.43333333333334,
                    },
                    KvaSizedAreaOption {
                        max_kva: 225.0,
                        area_sf: 130.17361111111106,
                    },
                    KvaSizedAreaOption {
                        max_kva: 300.0,
                        area_sf: 130.17361111111106,
                    },
                    KvaSizedAreaOption {
                        max_kva: 500.0,
                        area_sf: 154.171875,
                    },
                    KvaSizedAreaOption {
                        max_kva: 750.0,
                        area_sf: 157.90937499999998,
                    },
                    KvaSizedAreaOption {
                        max_kva: 1000.0,
                        area_sf: 159.77812499999999,
                    },
                    KvaSizedAreaOption {
                        max_kva: 1500.0,
                        area_sf: 185.00624999999999,
                    },
                    KvaSizedAreaOption {
                        max_kva: 2000.0,
                        area_sf: 158.92361111111109,
                    },
                    KvaSizedAreaOption {
                        max_kva: 2500.0,
                        area_sf: 168.50694444444443,
                    },
                    KvaSizedAreaOption {
                        max_kva: 3000.0,
                        area_sf: 188.72777777777776,
                    },
                    KvaSizedAreaOption {
                        max_kva: 3750.0,
                        area_sf: 196.77777777777777,
                    },
                    KvaSizedAreaOption {
                        max_kva: 5000.0,
                        area_sf: 217.86111111111114,
                    },
                    KvaSizedAreaOption {
                        max_kva: 7500.0,
                        area_sf: 231.72499999999999,
                    },
                ],
                electrical_service_input_voltage_v: 208.0,
                electrical_generator_output_voltage_v: 480.0,
                electrical_generator_room_enable_min_stories: 9,
                electrical_generator_site_factor: 0.95,
                electrical_generator_growth_ratio: 0.15,
                electrical_generator_fire_alarm_kva: 3.0,
                electrical_generator_smoke_control_kw: 40.0,
                electrical_generator_smoke_control_power_factor: 0.9,
                electrical_generator_smoke_control_demand_factor: 1.0,
                electrical_generator_domestic_booster_kw: 15.0,
                electrical_generator_domestic_booster_power_factor: 0.8,
                electrical_generator_domestic_booster_demand_factor: 0.5,
                electrical_generator_fire_pump_start_kva: 282.0,
                electrical_generator_emergency_start_kva: 170.0,
                electrical_generator_mandatory_start_kva: 270.0,
                electrical_generator_optional_start_kva: 114.0,
                electrical_generator_options: vec![
                    ElectricalGeneratorOption {
                        standby_rating_kva: 75.0,
                        installed_cost_usd: 32700.0,
                        added_clearance_footprint_sf: 128.3,
                    },
                    ElectricalGeneratorOption {
                        standby_rating_kva: 125.0,
                        installed_cost_usd: 37900.0,
                        added_clearance_footprint_sf: 141.6,
                    },
                    ElectricalGeneratorOption {
                        standby_rating_kva: 188.0,
                        installed_cost_usd: 44700.0,
                        added_clearance_footprint_sf: 160.5,
                    },
                    ElectricalGeneratorOption {
                        standby_rating_kva: 250.0,
                        installed_cost_usd: 51700.0,
                        added_clearance_footprint_sf: 175.0,
                    },
                    ElectricalGeneratorOption {
                        standby_rating_kva: 313.0,
                        installed_cost_usd: 59000.0,
                        added_clearance_footprint_sf: 174.2,
                    },
                    ElectricalGeneratorOption {
                        standby_rating_kva: 375.0,
                        installed_cost_usd: 66500.0,
                        added_clearance_footprint_sf: 187.8,
                    },
                    ElectricalGeneratorOption {
                        standby_rating_kva: 625.0,
                        installed_cost_usd: 99400.0,
                        added_clearance_footprint_sf: 224.8,
                    },
                    ElectricalGeneratorOption {
                        standby_rating_kva: 1000.0,
                        installed_cost_usd: 156700.0,
                        added_clearance_footprint_sf: 268.080694444444,
                    },
                    ElectricalGeneratorOption {
                        standby_rating_kva: 1250.0,
                        installed_cost_usd: 200300.0,
                        added_clearance_footprint_sf: 283.5,
                    },
                    ElectricalGeneratorOption {
                        standby_rating_kva: 1563.0,
                        installed_cost_usd: 260900.0,
                        added_clearance_footprint_sf: 329.7,
                    },
                    ElectricalGeneratorOption {
                        standby_rating_kva: 1875.0,
                        installed_cost_usd: 328000.0,
                        added_clearance_footprint_sf: 376.6,
                    },
                    ElectricalGeneratorOption {
                        standby_rating_kva: 2500.0,
                        installed_cost_usd: 482500.0,
                        added_clearance_footprint_sf: 376.6,
                    },
                ],
                electrical_ats_ev_qty: 0,
                electrical_ats_critical_qty: 0,
                electrical_ats_front_clear_ft: 3.0,
                electrical_ats_two_side_clear_ft: 2.0,
                electrical_ats_two_equipment_clear_ft: 1.0,
                electrical_ats_growth_ratio: 0.25,
                electrical_ats_step_down_small_max_kva: 500.0,
                electrical_ats_step_down_small_width_in: 54.0,
                electrical_ats_step_down_small_depth_in: 42.0,
                electrical_ats_step_down_large_width_in: 72.0,
                electrical_ats_step_down_large_depth_in: 60.0,
                electrical_ats_generator_distribution_width_in: 6.0,
                electrical_ats_generator_distribution_depth_in: 24.0,
                electrical_ats_service_entrance_breaker_width_in: 8.0,
                electrical_ats_service_entrance_breaker_depth_in: 8.0,
                electrical_ats_equipment_sizing: vec![
                    ElectricalAmpSizedEquipmentOption {
                        max_amps: 200.0,
                        width_in: 17.5,
                        depth_in: 35.0,
                    },
                    ElectricalAmpSizedEquipmentOption {
                        max_amps: 230.0,
                        width_in: 18.0,
                        depth_in: 14.3,
                    },
                    ElectricalAmpSizedEquipmentOption {
                        max_amps: 400.0,
                        width_in: 24.0,
                        depth_in: 18.2,
                    },
                    ElectricalAmpSizedEquipmentOption {
                        max_amps: 600.0,
                        width_in: 24.0,
                        depth_in: 18.2,
                    },
                    ElectricalAmpSizedEquipmentOption {
                        max_amps: 1000.0,
                        width_in: 34.0,
                        depth_in: 20.0,
                    },
                    ElectricalAmpSizedEquipmentOption {
                        max_amps: 1200.0,
                        width_in: 41.0,
                        depth_in: 33.5,
                    },
                    ElectricalAmpSizedEquipmentOption {
                        max_amps: 1600.0,
                        width_in: 42.5,
                        depth_in: 47.0,
                    },
                    ElectricalAmpSizedEquipmentOption {
                        max_amps: 2000.0,
                        width_in: 42.5,
                        depth_in: 47.0,
                    },
                    ElectricalAmpSizedEquipmentOption {
                        max_amps: 3000.0,
                        width_in: 41.0,
                        depth_in: 74.0,
                    },
                ],
                electrical_elir_enable_min_units: 50,
                electrical_elir_lighting_power_factor: 0.92,
                electrical_elir_room_guide: vec![
                    RuleOfThumbRoomGuidePoint {
                        load_kw: 5.0,
                        width_ft: 7.0,
                        depth_ft: 6.0,
                    },
                    RuleOfThumbRoomGuidePoint {
                        load_kw: 15.0,
                        width_ft: 9.0,
                        depth_ft: 6.0,
                    },
                    RuleOfThumbRoomGuidePoint {
                        load_kw: 30.0,
                        width_ft: 10.0,
                        depth_ft: 7.0,
                    },
                    RuleOfThumbRoomGuidePoint {
                        load_kw: 50.0,
                        width_ft: 14.0,
                        depth_ft: 6.0,
                    },
                    RuleOfThumbRoomGuidePoint {
                        load_kw: 100.0,
                        width_ft: 15.0,
                        depth_ft: 10.0,
                    },
                ],
                electrical_ups_room_enable_min_units: 50,
                electrical_ups_room_low_rise_max_stories: 3,
                electrical_ups_room_low_rise_min_available_roof_ratio: 0.15,
                electrical_ups_room_backup_time_hr: 0.8,
                electrical_ups_room_capacity_factor: 2.0,
                electrical_ups_room_battery_cabinet_kwh: 34.0,
                electrical_ups_room_battery_dod: 0.8,
                electrical_ups_room_battery_age_factor: 1.15,
                electrical_ups_room_power_cabinet_kwh: 250.0,
                electrical_ups_room_power_cabinet_qty_offset: 1,
                electrical_ups_room_pcs_kwh: 125.0,
                electrical_ups_room_distribution_cabinet_kwh: 250.0,
                electrical_ups_room_battery_width_in: 25.6,
                electrical_ups_room_battery_depth_in: 23.1,
                electrical_ups_room_power_width_in: 23.6,
                electrical_ups_room_power_depth_in: 37.5,
                electrical_ups_room_pcs_width_in: 23.6,
                electrical_ups_room_pcs_depth_in: 31.5,
                electrical_ups_room_distribution_width_in: 36.0,
                electrical_ups_room_distribution_depth_in: 32.0,
                electrical_ups_room_front_clear_ft: 3.0,
                electrical_ups_room_two_side_clear_ft: 2.0,
                electrical_ups_room_between_equipment_clear_ft: 1.0,
                electrical_ups_room_hvac_egress_ratio: 0.2,
                mechanical_ahu_room_enable_min_supply_air_cfm: 5000.0,
                mechanical_ahu_room_use_cono_sensor_control: false,
                mechanical_ahu_room_residential_air_cfm_per_sf: 0.03,
                mechanical_ahu_room_residential_air_cfm_per_bedroom: 7.5,
                mechanical_ahu_room_residential_bedroom_offset: 1.0,
                mechanical_ahu_room_non_residential_air_cfm_per_sf: 0.1,
                mechanical_ahu_room_parking_air_cfm_per_sf_full_on: 0.0,
                mechanical_ahu_room_parking_air_cfm_per_sf_sensor_control: 0.0,
                mechanical_ahu_room_width_base_ft: 7.0,
                mechanical_ahu_room_width_per_10k_cfm_ft: 0.5,
                mechanical_ahu_room_equipment_depth_base_ft: 3.0,
                mechanical_ahu_room_equipment_depth_per_sqrt_1k_cfm_ft: 0.35,
                mechanical_ahu_room_front_clear_ft: 4.0,
                mechanical_ahu_room_side_clear_ft: 2.0,
                mechanical_ventilation_riser_enable_min_supply_air_cfm: 5000.0,
                mechanical_ventilation_riser_small_units_max: 100,
                mechanical_ventilation_riser_small_density_sf_per_du: 30.0,
                mechanical_ventilation_riser_small_density_max_ratio: 0.30,
                mechanical_ventilation_riser_small_module_tons: 2.0,
                mechanical_ventilation_riser_small_module_qty_divisor: 6.0,
                mechanical_ventilation_riser_small_module_area_sf: 0.60546875,
                mechanical_ventilation_riser_large_module_coverage_sf: 7000.0,
                mechanical_ventilation_riser_large_module_area_sf: 0.24088541666666663,
                mechanical_ventilation_riser_residential_air_cfm_per_sf: 0.03,
                mechanical_ventilation_riser_residential_air_cfm_per_bedroom: 7.5,
                mechanical_ventilation_riser_residential_bedroom_offset: 1.0,
                mechanical_ventilation_riser_duct_velocity_fpm: 1400.0,
                mechanical_ventilation_riser_pipe_diameter_in: 1.0,
                mechanical_ventilation_riser_clearance_in: 2.0,
                mechanical_ventilation_riser_bathroom_exhaust_cfm: 50.0,
                mechanical_ventilation_riser_unit_kitchen_exhaust_cfm: 100.0,
                mechanical_ventilation_riser_bathrooms_per_studio: 1,
                mechanical_ventilation_riser_bathrooms_per_one_bedroom: 1,
                mechanical_ventilation_riser_bathrooms_per_two_bedroom: 2,
                mechanical_ventilation_riser_bathrooms_per_three_bedroom: 2,
                mechanical_pad_outdoor_enabled: false,
                mechanical_pad_outdoor_split_max_units: 100,
                mechanical_pad_outdoor_split_roof_sf_per_unit: 30.0,
                mechanical_pad_outdoor_split_max_roof_coverage_ratio: 0.30,
                mechanical_pad_outdoor_split_width_ft: 2.5,
                mechanical_pad_outdoor_split_depth_ft: 2.25,
                mechanical_pad_outdoor_vrf_tons: 14.0,
                mechanical_pad_outdoor_vrf_width_ft: 49.0 / 12.0,
                mechanical_pad_outdoor_vrf_depth_ft: 2.5,
                mechanical_pad_outdoor_residential_sf_per_ton: 500.0,
                mechanical_pad_outdoor_layout_divisor: 4.0,
                mechanical_pad_outdoor_service_aisle_ft: 3.0,
                mechanical_pad_outdoor_equipment_clear_ft: 2.0,
                mechanical_pad_outdoor_front_clear_ft: 4.0,
                mechanical_pad_outdoor_side_clear_ft: 3.0,
                commercial_kitchen_shaft_range_qty_units_1: 100.0,
                commercial_kitchen_shaft_range_qty_1: 1.0,
                commercial_kitchen_shaft_range_qty_units_2: 1000.0,
                commercial_kitchen_shaft_range_qty_2: 4.0,
                commercial_kitchen_shaft_range_width_ft: 3.0,
                commercial_kitchen_shaft_hood_length_offset_ft: 1.0,
                commercial_kitchen_shaft_use_hood_ul710: true,
                commercial_kitchen_shaft_range_type_electric: true,
                commercial_kitchen_shaft_ul710_exhaust_rate_cfm_per_ft: 250.0,
                commercial_kitchen_shaft_electric_exhaust_rate_cfm_per_ft: 300.0,
                commercial_kitchen_shaft_gas_exhaust_rate_cfm_per_ft: 400.0,
                commercial_kitchen_shaft_diversity_factor: 0.8,
                commercial_kitchen_shaft_future_expansion_ratio: 0.1,
                commercial_kitchen_shaft_exhaust_velocity_fpm: 1500.0,
                commercial_kitchen_shaft_round_duct_upsize_in: 2.0,
                commercial_kitchen_shaft_clearance_in: 6.0,
                commercial_kitchen_shaft_waste_vent_width_in: 4.0,
                commercial_kitchen_shaft_other_system_area_sf: 0.25,
                domestic_water_booster_room_enable_min_stories: 4,
                domestic_water_booster_room_story_height_ft: 10.5,
                domestic_water_booster_room_future_growth_ratio: 0.10,
                domestic_water_booster_room_continuous_demand_gpm: 10.0,
                domestic_water_booster_room_residential_wsfu_per_studio: 11.0,
                domestic_water_booster_room_residential_wsfu_per_one_bedroom: 11.0,
                domestic_water_booster_room_residential_wsfu_per_two_bedroom: 17.66,
                domestic_water_booster_room_residential_wsfu_per_three_bedroom: 18.25,
                domestic_water_booster_room_public_wc_wsfu: 5.0,
                domestic_water_booster_room_public_urinal_wsfu: 3.0,
                domestic_water_booster_room_public_lav_wsfu: 1.5,
                domestic_water_booster_room_public_kitchen_sink_qty: 2,
                domestic_water_booster_room_public_kitchen_sink_wsfu: 3.0,
                domestic_water_booster_room_public_service_sink_qty: 1,
                domestic_water_booster_room_public_service_sink_wsfu: 1.5,
                domestic_water_booster_room_peak_flow_cubic_a: 7.14989940834419e-10,
                domestic_water_booster_room_peak_flow_cubic_b: -0.000018476638375418,
                domestic_water_booster_room_peak_flow_cubic_c: 0.188701795436697,
                domestic_water_booster_room_peak_flow_cubic_d: 19.9557146947566,
                domestic_water_booster_room_pipe_velocity_fps: 8.0,
                domestic_water_booster_room_pipe_capacity_constant: 2.45,
                domestic_water_booster_room_hazen_williams_c: 140.0,
                domestic_water_booster_room_fitting_friction_loss_factor: 0.15,
                domestic_water_booster_room_friction_loss_max_psi_per_100ft: 5.0,
                domestic_water_booster_room_residual_pressure_psi: 20.0,
                domestic_water_booster_room_meter_backflow_prv_valves_psi: 20.0,
                domestic_water_booster_room_safety_margin_psi: 5.0,
                domestic_water_booster_room_lowest_city_pressure_psi: 40.0,
                domestic_water_booster_room_run_fraction: 0.30,
                domestic_water_booster_room_high_story_three_pump_min_stories: 12,
                domestic_water_booster_room_duty_pump_low_flow_max_gpm: 150.0,
                domestic_water_booster_room_duty_pump_mid_flow_max_gpm: 450.0,
                domestic_water_booster_room_low_story_max_for_longer_run: 9,
                domestic_water_booster_room_min_run_time_low_story_min: 0.5,
                domestic_water_booster_room_min_run_time_high_story_min: 0.25,
                domestic_water_booster_room_expansion_tank_delta_p_psi: 15.0,
                domestic_water_booster_room_expansion_tank_capacity_gal: 60.0,
                domestic_water_booster_room_expansion_tank_diameter_curve_a: 6.6,
                domestic_water_booster_room_expansion_tank_diameter_curve_b: 0.3,
                domestic_water_booster_room_expansion_tank_diameter_curve_c: 2.0,
                domestic_water_booster_room_standby_pump_qty: 1,
                domestic_water_booster_room_pump_width_in: 60.0,
                domestic_water_booster_room_pump_depth_in: 54.0,
                domestic_water_booster_room_control_panel_qty: 1,
                domestic_water_booster_room_control_panel_width_in: 30.0,
                domestic_water_booster_room_control_panel_depth_in: 12.0,
                domestic_water_booster_room_front_clear_ft: 3.0,
                domestic_water_booster_room_side_clear_ft: 0.0,
                domestic_water_booster_room_equipment_clear_ft: 2.0,
                cistern_water_storage_room_enabled: true,
                cistern_water_storage_room_a: 3.1287952735256601,
                cistern_water_storage_room_b: 168.27348005292785,
                backflow_preventer_room_enabled: true,
                backflow_preventer_max_pipe_diameter_in: 10.0,
                backflow_preventer_pipe_velocity_fps: 8.0,
                backflow_preventer_pipe_capacity_constant: 2.45,
                backflow_preventer_domestic_backflow_guide: vec![
                    DiameterSizedEquipmentOption {
                        max_diameter_in: 1.0,
                        width_in: 12.0,
                        depth_in: 48.0,
                    },
                    DiameterSizedEquipmentOption {
                        max_diameter_in: 2.0,
                        width_in: 21.0,
                        depth_in: 48.0,
                    },
                    DiameterSizedEquipmentOption {
                        max_diameter_in: 2.5,
                        width_in: 40.0,
                        depth_in: 48.0,
                    },
                    DiameterSizedEquipmentOption {
                        max_diameter_in: 3.0,
                        width_in: 50.0,
                        depth_in: 48.0,
                    },
                    DiameterSizedEquipmentOption {
                        max_diameter_in: 4.0,
                        width_in: 59.0,
                        depth_in: 48.0,
                    },
                    DiameterSizedEquipmentOption {
                        max_diameter_in: 6.0,
                        width_in: 69.0,
                        depth_in: 48.0,
                    },
                    DiameterSizedEquipmentOption {
                        max_diameter_in: 8.0,
                        width_in: 74.0,
                        depth_in: 48.0,
                    },
                    DiameterSizedEquipmentOption {
                        max_diameter_in: 10.0,
                        width_in: 86.0,
                        depth_in: 48.0,
                    },
                ],
                backflow_preventer_fire_backflow_guide: vec![
                    DiameterSizedEquipmentOption {
                        max_diameter_in: 3.0,
                        width_in: 48.0,
                        depth_in: 30.0,
                    },
                    DiameterSizedEquipmentOption {
                        max_diameter_in: 4.0,
                        width_in: 61.0,
                        depth_in: 36.0,
                    },
                    DiameterSizedEquipmentOption {
                        max_diameter_in: 6.0,
                        width_in: 74.0,
                        depth_in: 42.0,
                    },
                    DiameterSizedEquipmentOption {
                        max_diameter_in: 8.0,
                        width_in: 86.0,
                        depth_in: 42.0,
                    },
                    DiameterSizedEquipmentOption {
                        max_diameter_in: 10.0,
                        width_in: 90.0,
                        depth_in: 48.0,
                    },
                ],
                backflow_preventer_fire_backflow_qty: 1,
                backflow_preventer_irrigation_qty: 1,
                backflow_preventer_irrigation_width_in: 17.375,
                backflow_preventer_irrigation_depth_in: 5.125,
                backflow_preventer_front_clear_ft: 3.0,
                backflow_preventer_side_clear_ft: 0.0,
                backflow_preventer_equipment_clear_ft: 2.0,
                central_water_heating_room_indoor_enabled: true,
                central_water_heating_room_sum_width_ft: 16.197916666666668,
                central_water_heating_room_sum_depth_ft: 4.21875,
                central_water_heating_room_a: 3.2319785547045101,
                central_water_heating_room_b: 7.2912227109380279,
                central_water_heating_room_c: 1.9516446960384997,
                central_water_heating_pad_outdoor_enabled: false,
                central_water_heating_pad_outdoor_sum_width_ft: 16.197916666666668,
                central_water_heating_pad_outdoor_sum_depth_ft: 4.21875,
                central_water_heating_pad_outdoor_a: 3.2319785547045101,
                central_water_heating_pad_outdoor_b: 7.2912227109380279,
                central_water_heating_pad_outdoor_c: 1.9516446960384997,
                graywater_system_room_enabled: true,
                graywater_system_room_sum_width_ft: 14.75,
                graywater_system_room_sum_depth_ft: 3.791666666666667,
                graywater_system_room_a: -17.264919346786662,
                graywater_system_room_b: 84.090034759391955,
                graywater_system_room_c: 4.7230573603465018,
                gas_utility_room_enabled: true,
                gas_utility_master_meter_count: 1,
                gas_utility_single_meter_length_ft: 5.0,
                gas_utility_per_meter_length_ft: 2.0,
                gas_utility_length_offset_ft: 2.0,
                gas_utility_room_width_ft: 5.0,
                gas_utility_dcu_closet_sf: 36.0,
                gas_meter_space_alcove_enabled: false,
                gas_meter_space_alcove_bank_depth_ft: 3.0,
                gas_meter_space_alcove_front_clear_ft: 3.0,
                gas_meter_space_alcove_single_bank_width_ft: 14.0 / 12.0,
                gas_meter_space_alcove_two_meter_bank_width_ft: 34.0 / 12.0,
                gas_meter_space_alcove_additional_meter_width_ft: 16.0 / 12.0,
                gas_meter_space_alcove_side_clear_ft: 1.0,
                sub_electrical_enable_min_units: 60,
                sub_electrical_enable_min_gba_sf: 50_000.0,
                sub_electrical_units_per_panelboard: 12.0,
                sub_electrical_panel_depth_ft: 1.0,
                sub_electrical_front_clear_ft: 3.0,
                sub_electrical_wiggle_ft: 0.5,
                sub_electrical_panel_width_ft: 2.5,
                sub_electrical_side_clear_ft: 2.0,
                sub_electrical_growth_ratio: 0.2,
                sub_electrical_option_1_max_units_per_floor: 60.0,
                sub_electrical_option_2_max_units_per_floor: 120.0,
                sub_electrical_option_2_w_ft: 10.0,
                sub_electrical_option_2_d_ft: 10.0,
                sub_electrical_option_3_w_ft: 10.0,
                sub_electrical_option_3_d_ft: 15.0,
                electrical_main_room_enable_min_units: 16,
                electrical_elevator_kva_per_car: 20.0,
                electrical_motor_constant_kva: 63.037,
                electrical_corridor_w_per_sf: 5.5,
                electrical_retail_w_per_sf: 22.5,
                electrical_shaft_stair_w_per_sf: 6.5,
                electrical_entry_lobby_w_per_sf: 22.5,
                electrical_indoor_parking_w_per_sf: 1.25,
                electrical_residential_lighting_w_per_sf: 3.0,
                electrical_dwelling_unit_appliance_connected_kva_per_du: 20.45,
                electrical_dwelling_unit_misc_connected_kva_per_du: 2.267,
                electrical_stair_shaft_area_sf_per_stair_per_floor: 84.5079365079365,
            },
            solver: SolverAssumption {
                unit_size_quantile_max_far: 0.40,
                unit_size_quantile_max_du: 0.18,
                unit_size_quantile_balanced_yield: 0.52,
                repeatability_weight_default: 0.85,
            },
            preliminary_area: PreliminaryAreaAssumption {
                default_affordable_support_profile: true,
                support_staff_ratio_affordable: PreliminaryAreaRatioFormula {
                    slope_inv_stories: -0.0296562344750819,
                    slope_inv_gfa: 2020.30988757719,
                    slope_story_over_gfa: 21.4875531529969,
                    intercept: 0.0463767794258712,
                },
                support_staff_ratio_market: PreliminaryAreaRatioFormula {
                    slope_inv_stories: -0.0292965503799396,
                    slope_inv_gfa: 1745.97020664942,
                    slope_story_over_gfa: 22.2992695811091,
                    intercept: 0.0429088168923499,
                },
                coarse_corridor_gfa_break_sf: 25_000.0,
                coarse_corridor_width_low_ft: 4.0,
                coarse_corridor_width_high_ft: 5.0,
                coarse_corridor_loading_factor: 2.0,
                fixed_common_area_sf: 70.0,
                stair_vestibule_area_sf: 40.0,
                stair_vestibule_count_per_floor: 2.0,
                stair_occ_normalizer_sf: 100.0,
                elevator_lobby_per_bank_low_mid_sf: 120.0,
                elevator_lobby_per_bank_high_sf: 150.0,
                elevator_lobby_split_factor: 0.5,
                elevator_lobby_normalizer_sf: 80.0,
                open_space_weight: 0.25,
                open_space_sf_studio: 100.0,
                open_space_sf_one_bed: 100.0,
                open_space_sf_two_bed: 125.0,
                open_space_sf_three_bed: 175.0,
                circulation_small_gba_threshold_sf: 27_250.0,
                circulation_large_gba_threshold_sf: 2_618_550.0,
                circulation_small_ratio: 0.1831,
                circulation_large_ratio: 0.0801,
                circulation_small_quad_a: -3.23685901158893E-08,
                circulation_small_quad_b: 0.0912391736085469,
                circulation_small_quad_c: 2528.21139403587,
                circulation_small_min_sf: 3783.0,
                circulation_small_max_sf: 59223.0,
                circulation_large_quad_a: 1.60103331795005E-08,
                circulation_large_quad_b: 0.0347439254150707,
                circulation_large_quad_c: 8982.97329958147,
                circulation_large_min_sf: 48009.25,
                circulation_large_max_sf: 209787.0,
            },
        }
    }
}

pub fn shape_coverage_ratio(shape: BuildingShape) -> f64 {
    match shape {
        BuildingShape::Bar => 0.52,
        BuildingShape::LShape => 0.58,
        BuildingShape::UShape => 0.63,
        BuildingShape::OShape => 0.72,
        BuildingShape::HShape => 0.60,
        BuildingShape::Tower => 0.34,
        BuildingShape::XShape => 0.40,
        BuildingShape::Cluster => 0.48,
        BuildingShape::FreeForm => 0.50,
        BuildingShape::PerimeterPartial => 0.68,
    }
}

pub fn construction_multiplier(t: BuildingConstructionType) -> f64 {
    match t {
        BuildingConstructionType::TypeV => 0.95,
        BuildingConstructionType::TypeIII => 1.00,
        BuildingConstructionType::TypeVOverI => 1.06,
        BuildingConstructionType::TypeIIIOverI => 1.08,
        BuildingConstructionType::TypeI => 1.12,
    }
}

/* ============================== code profiles ============================= */

#[derive(Debug, Clone)]
pub struct JurisdictionCodePack {
    pub base_code_edition: BaseCodeEdition,
    pub state_amendment_profile: StateAmendmentProfile,
    pub min_corridor_clear_width_ft: f64,
    pub occupant_load_factor_residential_sf_per_occ: f64,
    pub exit_access_travel_sprinklered_ft: f64,
    pub stair_width_per_occ_in: f64,
    pub stair_width_min_in: f64,
    pub retail_sf_per_stall: f64,
    pub surface_parking_area_ratio: f64,
    pub podium_allowed: bool,
    pub amendment_tags: Vec<String>,
}

impl JurisdictionCodePack {
    pub fn from_input(input: &LayoutInput) -> Self {
        let mut out = Self {
            base_code_edition: input.jurisdiction_profile.base_code_edition,
            state_amendment_profile: input.jurisdiction_profile.state_amendment_profile,
            min_corridor_clear_width_ft: 4.0,
            occupant_load_factor_residential_sf_per_occ: 200.0,
            exit_access_travel_sprinklered_ft: 250.0,
            stair_width_per_occ_in: 0.30,
            stair_width_min_in: 44.0,
            retail_sf_per_stall: 300.0,
            surface_parking_area_ratio: 0.92,
            podium_allowed: matches!(
                input.building_construction_type,
                BuildingConstructionType::TypeVOverI | BuildingConstructionType::TypeIIIOverI
            ),
            amendment_tags: Vec::new(),
        };

        match input.jurisdiction_profile.base_code_edition {
            BaseCodeEdition::Ibc2021 => out.amendment_tags.push("ibc_2021".to_string()),
            BaseCodeEdition::Ibc2024 => {
                out.amendment_tags.push("ibc_2024".to_string());
                out.exit_access_travel_sprinklered_ft =
                    out.exit_access_travel_sprinklered_ft.max(250.0);
            }
        }

        match input.jurisdiction_profile.state_amendment_profile {
            StateAmendmentProfile::None => {}
            StateAmendmentProfile::Ca2022Title24 => {
                out.amendment_tags.push("ca_2022_title24".to_string());
                out.min_corridor_clear_width_ft = out.min_corridor_clear_width_ft.max(4.0);
                out.retail_sf_per_stall = 500.0;
                out.surface_parking_area_ratio = 0.88;
            }
            StateAmendmentProfile::Ca2025Title24 => {
                out.amendment_tags.push("ca_2025_title24".to_string());
                out.min_corridor_clear_width_ft = out.min_corridor_clear_width_ft.max(4.0);
                out.retail_sf_per_stall = 550.0;
                out.surface_parking_area_ratio = 0.86;
            }
            StateAmendmentProfile::Fl2023Fbc => {
                out.amendment_tags.push("fl_2023_fbc".to_string());
                out.retail_sf_per_stall = 420.0;
                out.surface_parking_area_ratio = 0.90;
            }
        }

        if let Some(ov) = &input.code_profile_overrides {
            if let Some(v) = ov.min_corridor_clear_width_ft {
                out.min_corridor_clear_width_ft = v;
            }
            if let Some(v) = ov.occupant_load_factor_residential_sf_per_occ {
                out.occupant_load_factor_residential_sf_per_occ = v;
            }
            if let Some(v) = ov.exit_access_travel_sprinklered_ft {
                out.exit_access_travel_sprinklered_ft = v;
            }
            if let Some(v) = ov.stair_width_per_occ_in {
                out.stair_width_per_occ_in = v;
            }
            if let Some(v) = ov.stair_width_min_in {
                out.stair_width_min_in = v;
            }
            if let Some(v) = ov.retail_sf_per_stall {
                out.retail_sf_per_stall = v;
            }
            if let Some(v) = ov.surface_parking_area_ratio {
                out.surface_parking_area_ratio = v;
            }
            out.amendment_tags.push("code_profile_override".to_string());
        }

        out
    }
}

/* ============================== formula layer ============================= */

pub fn building_height_category(stories: u32, a: &AssumptionPack) -> &'static str {
    if stories <= a.vertical.low_rise_max_stories {
        "low_rise"
    } else if stories <= a.vertical.mid_rise_max_stories {
        "mid_rise"
    } else {
        "high_rise"
    }
}

pub fn in_unit_wd_ratio(input: &NormalizedInput) -> f64 {
    match input.residential_features.in_unit_wd.mode {
        InUnitWdMode::AllUnits => 1.0,
        InUnitWdMode::None => 0.0,
        InUnitWdMode::Partial => input
            .residential_features
            .in_unit_wd
            .partial_ratio
            .unwrap_or(0.5),
        InUnitWdMode::Auto => 0.75,
    }
}

pub fn objective_size_quantile(obj: OptimizationObjective, a: &AssumptionPack) -> f64 {
    match obj {
        OptimizationObjective::MaximizeFar => a.solver.unit_size_quantile_max_far,
        OptimizationObjective::MaximizeDwellingUnits => a.solver.unit_size_quantile_max_du,
        OptimizationObjective::MaximizeBalancedYield => a.solver.unit_size_quantile_balanced_yield,
    }
}

pub fn resolved_unit_area_sf(input: &NormalizedInput, t: UnitType, a: &AssumptionPack) -> f64 {
    let band = a.unit_size_targets_sf.get(t);
    let mut q = objective_size_quantile(input.optimization.objective, a);

    if !input.optimization.allow_unit_size_rebalance {
        q = if (band.max_sf - band.min_sf).abs() <= EPS {
            0.5
        } else {
            (band.target_sf - band.min_sf) / (band.max_sf - band.min_sf)
        };
    }

    let with_wd = lerp(q, 0.0, band.min_sf, 1.0, band.max_sf);
    let without_wd = lerp(q, 0.0, band.min_sf - 10.0, 1.0, band.max_sf - 25.0);
    let r = in_unit_wd_ratio(input);
    let mut resolved_sf = r * with_wd + (1.0 - r) * without_wd;

    if !input.optimization.allow_unit_size_rebalance {
        resolved_sf = band.target_sf;
    }

    clamp(resolved_sf, band.min_sf, band.max_sf)
}

pub fn resolved_depth_over_width_ratio(t: UnitType, obj: OptimizationObjective) -> f64 {
    let q = match obj {
        OptimizationObjective::MaximizeFar => 0.40,
        OptimizationObjective::MaximizeDwellingUnits => 0.18,
        OptimizationObjective::MaximizeBalancedYield => 0.52,
    };

    let (r0, r1) = match t {
        UnitType::Studio => (1.50, 2.00),
        UnitType::OneBedroom => (1.10, 1.20),
        UnitType::TwoBedroom => (0.50, 0.80),
        UnitType::ThreeBedroom => (0.65, 0.70),
    };

    lerp(q, 0.0, r0, 1.0, r1)
}

pub fn unit_width_depth_ft(
    area_sf: f64,
    depth_over_width: f64,
    input: &NormalizedInput,
    a: &AssumptionPack,
) -> (f64, f64) {
    let mut w = (area_sf / depth_over_width).sqrt();
    let mut d = (area_sf * depth_over_width).sqrt();

    let depth_cap = if input.constraints.min_daylight {
        input
            .constraints
            .max_unit_depth_ft
            .min(a.geometry.daylight_depth_cap_ft)
    } else {
        input.constraints.max_unit_depth_ft
    };

    d = d.min(depth_cap);
    w = area_sf / d;
    w = w.max(a.geometry.unit_rect_min_width_ft);
    d = d.max(a.geometry.unit_rect_min_depth_ft);

    (w, d)
}

/* workbook-inherited primitive */
pub fn f_flat(values: &[f64], counts: &[usize]) -> Vec<f64> {
    let mut out = Vec::<f64>::new();
    for (value, count) in values.iter().zip(counts.iter()) {
        for _ in 0..*count {
            out.push(*value);
        }
    }
    out
}

/* workbook-inherited primitive */
pub fn f_roomsize(
    qty: &[usize],
    w_ft: &[f64],
    d_ft: &[f64],
    clr_front: f64,
    clr_2side: f64,
    clr_2equip: f64,
) -> (f64, f64) {
    let total_half = w_ft
        .iter()
        .zip(qty.iter())
        .map(|(w, q)| *w * *q as f64)
        .sum::<f64>()
        / 2.0;

    let flat_w = f_flat(w_ft, qty);
    if flat_w.is_empty() {
        return (0.0, 0.0);
    }

    // Workbook LAMBDA uses MATCH(half_width, running_sums, 1), which picks the
    // last cumulative width that is still <= half_width.
    let mut split_idx = 1usize;
    for (i, x) in flat_w.iter().enumerate() {
        let running = flat_w.iter().take(i + 1).sum::<f64>();
        if running <= total_half + EPS {
            split_idx = i + 1;
        } else {
            break;
        }
    }

    let left_sum: f64 = flat_w.iter().take(split_idx).sum();
    let right_sum: f64 = flat_w.iter().skip(split_idx).sum();
    let tew = left_sum.max(right_sum);
    let tce = clr_2equip * (split_idx.saturating_sub(1) as f64);
    let room_width = tew + tce + clr_2side;

    let flat_d = f_flat(d_ft, qty);
    let left_d = flat_d
        .iter()
        .take(split_idx)
        .fold(0.0_f64, |a, b| a.max(*b));
    let right_d = flat_d
        .iter()
        .skip(split_idx)
        .fold(0.0_f64, |a, b| a.max(*b));
    let room_depth = left_d + right_d + clr_front;

    (room_width, room_depth)
}

pub fn allocate_integer_unit_counts(total: u32, mix: &UnitMix) -> [u32; 4] {
    let ratios = [
        mix.studio,
        mix.one_bedroom,
        mix.two_bedroom,
        mix.three_bedroom,
    ];
    let mut raw = [0.0; 4];
    let mut base = [0u32; 4];
    let mut used = 0u32;

    for i in 0..4 {
        raw[i] = ratios[i] * total as f64;
        base[i] = raw[i].floor().max(0.0) as u32;
        used += base[i];
    }

    let mut residual = total.saturating_sub(used);
    let mut idx = vec![0usize, 1usize, 2usize, 3usize];
    idx.sort_by(|&a, &b| {
        let fa = raw[a] - raw[a].floor();
        let fb = raw[b] - raw[b].floor();
        fb.partial_cmp(&fa).unwrap_or(Ordering::Equal)
    });

    let mut k = 0usize;
    while residual > 0 {
        base[idx[k % 4]] += 1;
        residual -= 1;
        k += 1;
    }

    base
}

pub fn weighted_average_unit_area_sf(counts: [u32; 4], areas: [f64; 4]) -> f64 {
    let du = counts.iter().copied().sum::<u32>() as f64;
    if du <= EPS {
        return 0.0;
    }
    let a = counts[0] as f64 * areas[0]
        + counts[1] as f64 * areas[1]
        + counts[2] as f64 * areas[2]
        + counts[3] as f64 * areas[3];
    a / du
}

pub fn preliminary_mix_weights(mix: &UnitMix) -> [f64; 4] {
    let total = (mix.studio + mix.one_bedroom + mix.two_bedroom + mix.three_bedroom).max(EPS);
    [
        mix.studio / total,
        mix.one_bedroom / total,
        mix.two_bedroom / total,
        mix.three_bedroom / total,
    ]
}

pub fn preliminary_avg_unit_area_sf_from_mix(
    mix: &UnitMix,
    input: &NormalizedInput,
    a: &AssumptionPack,
) -> f64 {
    let weights = preliminary_mix_weights(mix);
    let unit_types = [
        UnitType::Studio,
        UnitType::OneBedroom,
        UnitType::TwoBedroom,
        UnitType::ThreeBedroom,
    ];
    unit_types
        .iter()
        .copied()
        .enumerate()
        .map(|(i, unit_type)| weights[i] * resolved_unit_area_sf(input, unit_type, a))
        .sum::<f64>()
}

pub fn preliminary_support_staff_ratio(stories: u32, gfa_sf: f64, a: &AssumptionPack) -> f64 {
    let formula = if a.preliminary_area.default_affordable_support_profile {
        &a.preliminary_area.support_staff_ratio_affordable
    } else {
        &a.preliminary_area.support_staff_ratio_market
    };
    let stories_f = stories.max(1) as f64;
    let gfa = gfa_sf.max(1.0);
    (
        formula.slope_inv_stories / stories_f
            + formula.slope_inv_gfa / gfa
            + formula.slope_story_over_gfa * stories_f / gfa
            + formula.intercept
    )
    .clamp(0.0, 0.35)
}

pub fn preliminary_corridor_width_ft(gfa_sf: f64, a: &AssumptionPack) -> f64 {
    if gfa_sf < a.preliminary_area.coarse_corridor_gfa_break_sf {
        a.preliminary_area.coarse_corridor_width_low_ft
    } else {
        a.preliminary_area.coarse_corridor_width_high_ft
    }
}

pub fn preliminary_open_space_req_sf_per_unit(t: UnitType, a: &AssumptionPack) -> f64 {
    match t {
        UnitType::Studio => a.preliminary_area.open_space_sf_studio,
        UnitType::OneBedroom => a.preliminary_area.open_space_sf_one_bed,
        UnitType::TwoBedroom => a.preliminary_area.open_space_sf_two_bed,
        UnitType::ThreeBedroom => a.preliminary_area.open_space_sf_three_bed,
    }
}

pub fn preliminary_area_budget_from_mix(
    gfa_sf: f64,
    retail_area_sf: f64,
    stories: u32,
    mix: &UnitMix,
    input: &NormalizedInput,
    a: &AssumptionPack,
) -> PreliminaryAreaBudget {
    // This is intentionally a workbook-style coarse rectangle model: it carries
    // corridor, lobby, stair, and open-space pressure forward before exact layer
    // geometry exists, but it does not try to replace the later detailed solve.
    let weights = preliminary_mix_weights(mix);
    let unit_types = [
        UnitType::Studio,
        UnitType::OneBedroom,
        UnitType::TwoBedroom,
        UnitType::ThreeBedroom,
    ];
    let mut areas = [0.0; 4];
    let mut widths = [0.0; 4];
    let mut open_space = [0.0; 4];

    for (i, unit_type) in unit_types.iter().copied().enumerate() {
        let area = resolved_unit_area_sf(input, unit_type, a);
        let ratio = resolved_depth_over_width_ratio(unit_type, input.optimization.objective);
        let (width, _) = unit_width_depth_ft(area, ratio, input, a);
        areas[i] = area;
        widths[i] = width;
        open_space[i] = preliminary_open_space_req_sf_per_unit(unit_type, a);
    }

    let avg_unit_area_sf = weights
        .iter()
        .zip(areas.iter())
        .map(|(weight, area)| weight * area)
        .sum::<f64>();

    let mut alpha = 0.0;
    let mut beta = 0.0;
    let mut gamma = 0.0;
    for i in 0..4 {
        let area = areas[i].max(1.0);
        alpha += weights[i] / area;
        beta += weights[i] * widths[i] / area;
        gamma += weights[i] * open_space[i] / area;
    }

    let support_staff_ratio = preliminary_support_staff_ratio(stories, gfa_sf, a);
    let support_staff_area_sf = support_staff_ratio * (gfa_sf - retail_area_sf).max(0.0);
    let corridor_width_ft = preliminary_corridor_width_ft(gfa_sf, a);
    let corridor_term = corridor_width_ft * beta
        / a.preliminary_area.coarse_corridor_loading_factor.max(1.0);
    let elevator_lobby_area_sf = if stories <= a.vertical.mid_rise_max_stories {
        a.preliminary_area.elevator_lobby_per_bank_low_mid_sf
    } else {
        a.preliminary_area.elevator_lobby_per_bank_high_sf
    };
    let elevator_lobby_term = a.preliminary_area.elevator_lobby_split_factor
        * elevator_lobby_area_sf
        * alpha
        / a.preliminary_area.elevator_lobby_normalizer_sf.max(EPS);
    let stair_vestibule_total_sf = a.preliminary_area.stair_vestibule_area_sf
        * a.preliminary_area.stair_vestibule_count_per_floor
        * stories.max(1) as f64;
    let stair_term = a.preliminary_area.stair_vestibule_area_sf * stories.max(1) as f64 * alpha
        / a.preliminary_area.stair_occ_normalizer_sf.max(EPS);
    let open_space_term = a.preliminary_area.open_space_weight * gamma;
    let denominator = 1.0 + corridor_term + elevator_lobby_term + stair_term + open_space_term;
    let fixed_common_area_sf = a.preliminary_area.fixed_common_area_sf;
    let residential_area_sf = ((((1.0 - support_staff_ratio) * (gfa_sf - retail_area_sf).max(0.0))
        - (fixed_common_area_sf + stair_vestibule_total_sf))
        / denominator.max(EPS))
    .max(0.0);
    let gba_est_sf = gfa_sf + support_staff_area_sf + fixed_common_area_sf + stair_vestibule_total_sf;
    let circulation_area_sf = circulation_area_regression_sf(gba_est_sf, a);
    let weighted_open_space_sf_per_unit = weights
        .iter()
        .zip(open_space.iter())
        .map(|(weight, open_space_sf)| weight * open_space_sf)
        .sum::<f64>();

    PreliminaryAreaBudget {
        avg_unit_area_sf,
        support_staff_ratio,
        support_staff_area_sf,
        corridor_width_ft,
        corridor_term,
        elevator_lobby_term,
        stair_term,
        open_space_term,
        fixed_common_area_sf,
        stair_vestibule_total_sf,
        circulation_area_sf,
        residential_area_sf,
        weighted_open_space_sf_per_unit,
    }
}

pub fn preliminary_dwelling_units_from_gfa(
    gfa_sf: f64,
    retail_area_sf: f64,
    stories: u32,
    mix: &UnitMix,
    input: &NormalizedInput,
    a: &AssumptionPack,
) -> u32 {
    let budget =
        preliminary_area_budget_from_mix(gfa_sf, retail_area_sf, stories, mix, input, a);
    (budget.residential_area_sf / budget.avg_unit_area_sf.max(1.0))
        .floor()
        .max(1.0) as u32
}

pub fn preliminary_gfa_from_dwelling_units(
    dwelling_units: u32,
    retail_area_sf: f64,
    stories: u32,
    mix: &UnitMix,
    input: &NormalizedInput,
    a: &AssumptionPack,
) -> f64 {
    let target_residential_area_sf = dwelling_units as f64
        * preliminary_avg_unit_area_sf_from_mix(mix, input, a).max(1.0);
    let mut gfa_sf = retail_area_sf + target_residential_area_sf;

    for _ in 0..8 {
        let budget =
            preliminary_area_budget_from_mix(gfa_sf, retail_area_sf, stories, mix, input, a);
        let denominator =
            1.0 + budget.corridor_term + budget.elevator_lobby_term + budget.stair_term + budget.open_space_term;
        let next = retail_area_sf
            + (target_residential_area_sf
                * denominator
                + budget.fixed_common_area_sf
                + budget.stair_vestibule_total_sf)
                / (1.0 - budget.support_staff_ratio).max(EPS);
        if (next - gfa_sf).abs() <= 1.0e-3 {
            gfa_sf = next;
            break;
        }
        gfa_sf = next;
    }

    gfa_sf.max(retail_area_sf)
}

pub fn circulation_area_regression_sf(gba_sf: f64, a: &AssumptionPack) -> f64 {
    let x = gba_sf.max(0.0);
    let p = &a.preliminary_area;
    if x < p.circulation_small_gba_threshold_sf {
        return p.circulation_small_ratio * x;
    }
    if x > p.circulation_large_gba_threshold_sf {
        return p.circulation_large_ratio * x;
    }
    if x < 800_000.0 {
        let y = p.circulation_small_quad_a * x * x
            + p.circulation_small_quad_b * x
            + p.circulation_small_quad_c;
        return y.clamp(p.circulation_small_min_sf, p.circulation_small_max_sf);
    }
    let y = p.circulation_large_quad_a * x * x
        + p.circulation_large_quad_b * x
        + p.circulation_large_quad_c;
    y.clamp(p.circulation_large_min_sf, p.circulation_large_max_sf)
}

#[derive(Debug, Clone)]
pub struct UnitMixOptimizationResult {
    pub total_units: u32,
    pub counts: [u32; 4],
    pub mix: UnitMix,
    pub score_total: f64,
    pub score_economic: f64,
    pub score_focus: f64,
    pub avg_unit_area_sf: f64,
}

pub fn rank_of(values: &[f64], idx: usize, ascending: bool) -> f64 {
    let mut pairs: Vec<(usize, f64)> = values.iter().copied().enumerate().collect();
    pairs.sort_by(|a, b| {
        if ascending {
            a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal)
        } else {
            b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
        }
    });

    for (rank, (j, _)) in pairs.iter().enumerate() {
        if *j == idx {
            return (rank + 1) as f64;
        }
    }
    values.len() as f64
}

pub fn softmax_mix_search(
    net_res_area_sf: f64,
    input: &NormalizedInput,
    a: &AssumptionPack,
) -> UnitMixOptimizationResult {
    let areas = [
        resolved_unit_area_sf(input, UnitType::Studio, a),
        resolved_unit_area_sf(input, UnitType::OneBedroom, a),
        resolved_unit_area_sf(input, UnitType::TwoBedroom, a),
        resolved_unit_area_sf(input, UnitType::ThreeBedroom, a),
    ];

    let min_area = areas.iter().fold(f64::INFINITY, |x, y| x.min(*y));
    let max_area = areas.iter().fold(0.0_f64, |x, y| x.max(*y));
    let q_max = (net_res_area_sf / min_area).floor().max(1.0) as u32;
    let q_min = (net_res_area_sf / max_area).ceil().max(1.0) as u32;

    let rents_per_sf = [
        a.economics.rent_per_sf_studio,
        a.economics.rent_per_sf_one_bed,
        a.economics.rent_per_sf_two_bed,
        a.economics.rent_per_sf_three_bed,
    ];
    let costs_per_sf = [
        a.economics.cost_per_sf_studio,
        a.economics.cost_per_sf_one_bed,
        a.economics.cost_per_sf_two_bed,
        a.economics.cost_per_sf_three_bed,
    ];

    let rent_room = [
        rents_per_sf[0] * areas[0],
        rents_per_sf[1] * areas[1],
        rents_per_sf[2] * areas[2],
        rents_per_sf[3] * areas[3],
    ];
    let cost_room = [
        costs_per_sf[0] * areas[0],
        costs_per_sf[1] * areas[1],
        costs_per_sf[2] * areas[2],
        costs_per_sf[3] * areas[3],
    ];

    let mut econ_scores = Vec::<f64>::new();
    let mut focus_scores = Vec::<f64>::new();
    let mut results = Vec::<UnitMixOptimizationResult>::new();

    for q in q_min..=q_max {
        let target_avg = net_res_area_sf / q as f64;
        let base_scores = [
            rent_room[0] / cost_room[0].max(1.0),
            rent_room[1] / cost_room[1].max(1.0),
            rent_room[2] / cost_room[2].max(1.0),
            rent_room[3] / cost_room[3].max(1.0),
        ];

        let mut beta = 0.0_f64;
        for _ in 0..12 {
            let expv = [
                (beta * base_scores[0]).exp(),
                (beta * base_scores[1]).exp(),
                (beta * base_scores[2]).exp(),
                (beta * base_scores[3]).exp(),
            ];
            let z = expv.iter().sum::<f64>().max(EPS);
            let p = [expv[0] / z, expv[1] / z, expv[2] / z, expv[3] / z];
            let e_area = p[0] * areas[0] + p[1] * areas[1] + p[2] * areas[2] + p[3] * areas[3];
            let e_score = p[0] * base_scores[0]
                + p[1] * base_scores[1]
                + p[2] * base_scores[2]
                + p[3] * base_scores[3];
            let e_area_score = p[0] * areas[0] * base_scores[0]
                + p[1] * areas[1] * base_scores[1]
                + p[2] * areas[2] * base_scores[2]
                + p[3] * areas[3] * base_scores[3];
            let cov = e_area_score - e_area * e_score;
            let f = e_area - target_avg;
            if f.abs() <= 1.0e-3 || cov.abs() <= 1.0e-6 {
                break;
            }
            beta -= f / cov;
            beta = clamp(beta, -12.0, 12.0);
        }

        let expv = [
            (beta * base_scores[0]).exp(),
            (beta * base_scores[1]).exp(),
            (beta * base_scores[2]).exp(),
            (beta * base_scores[3]).exp(),
        ];
        let z = expv.iter().sum::<f64>().max(EPS);
        let p = [expv[0] / z, expv[1] / z, expv[2] / z, expv[3] / z];

        let mix = UnitMix {
            studio: p[0],
            one_bedroom: p[1],
            two_bedroom: p[2],
            three_bedroom: p[3],
        };

        let counts = allocate_integer_unit_counts(q, &mix);
        let avg_area = weighted_average_unit_area_sf(counts, areas);
        let total_revenue = (12.0 - a.economics.vacancy_months_per_year)
            * (counts[0] as f64 * rent_room[0]
                + counts[1] as f64 * rent_room[1]
                + counts[2] as f64 * rent_room[2]
                + counts[3] as f64 * rent_room[3]);
        let total_cost = counts[0] as f64 * cost_room[0]
            + counts[1] as f64 * cost_room[1]
            + counts[2] as f64 * cost_room[2]
            + counts[3] as f64 * cost_room[3];
        let economic = a.economics.revenue_years * total_revenue / total_cost.max(1.0);

        let mean_p = 0.25;
        let focus = ((p[0] - mean_p).powi(2)
            + (p[1] - mean_p).powi(2)
            + (p[2] - mean_p).powi(2)
            + (p[3] - mean_p).powi(2))
            / 4.0;

        econ_scores.push(economic);
        focus_scores.push(focus);
        results.push(UnitMixOptimizationResult {
            total_units: q,
            counts,
            mix,
            score_total: 0.0,
            score_economic: economic,
            score_focus: focus,
            avg_unit_area_sf: avg_area,
        });
    }

    for i in 0..results.len() {
        let econ_rank = rank_of(&econ_scores, i, false);
        let focus_rank = rank_of(&focus_scores, i, false);
        let total = a.economics.efficiency_weight * econ_rank
            + (1.0 - a.economics.efficiency_weight) * focus_rank;
        results[i].score_total = total;
    }

    results.sort_by(|a, b| {
        a.score_total
            .partial_cmp(&b.score_total)
            .unwrap_or(Ordering::Equal)
    });
    results
        .into_iter()
        .next()
        .unwrap_or(UnitMixOptimizationResult {
            total_units: 1,
            counts: [0, 1, 0, 0],
            mix: UnitMix {
                studio: 0.0,
                one_bedroom: 1.0,
                two_bedroom: 0.0,
                three_bedroom: 0.0,
            },
            score_total: 0.0,
            score_economic: 0.0,
            score_focus: 0.0,
            avg_unit_area_sf: areas[1],
        })
}

/* ========================== support / amenity math ======================== */

pub fn bicycle_short_term_spaces(units: u32) -> u32 {
    let u = units as f64;
    if units <= 25 {
        (u / 10.0).ceil() as u32
    } else if units <= 100 {
        (u / 15.0).ceil() as u32
    } else if units <= 200 {
        (u / 20.0).ceil() as u32
    } else {
        (u / 40.0).ceil() as u32
    }
}

pub fn bicycle_long_term_spaces(units: u32) -> u32 {
    let u = units as f64;
    if units <= 25 {
        (u / 1.0).ceil() as u32
    } else if units <= 100 {
        (u / 1.5).ceil() as u32
    } else if units <= 200 {
        (u / 2.0).ceil() as u32
    } else {
        (u / 4.0).ceil() as u32
    }
}

pub fn bicycle_repair_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    let long_term = bicycle_long_term_spaces(units);
    if long_term < a.support.bicycle_repair_area_long_term_stall_threshold {
        return 0.0;
    }
    let qty = interpolated_count_from_units(
        units,
        a.support.bicycle_repair_units_1,
        a.support.bicycle_repair_qty_1,
        a.support.bicycle_repair_units_2,
        a.support.bicycle_repair_qty_2,
    );
    qty.max(1) as f64 * a.support.bicycle_repair_area_sf
}

pub fn bicycle_room_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    let stalls = bicycle_short_term_spaces(units) + bicycle_long_term_spaces(units);
    stalls as f64
        * (a.support.bicycle_rack_length_ft + a.support.bicycle_rack_aisle_ft)
        * a.support.bicycle_rack_width_ft
}

pub fn common_laundry_area_sf(units: u32, in_unit_wd_ratio: f64, a: &AssumptionPack) -> f64 {
    let du_without_wd = (units as f64 * (1.0 - in_unit_wd_ratio)).round() as u32;
    if du_without_wd == 0 {
        return 0.0;
    }
    let pairs = (du_without_wd as f64 / 10.0).ceil();
    let aux = lerp(
        units as f64,
        a.support.common_laundry_aux_units_1,
        a.support.common_laundry_aux_sf_1,
        a.support.common_laundry_aux_units_2,
        a.support.common_laundry_aux_sf_2,
    )
    .round();
    let pair_area = a.support.common_laundry_pair_w_ft * a.support.common_laundry_pair_d_ft
        + a.support.common_laundry_pair_clearance_sf;
    (pairs * pair_area + aux).max(a.support.common_laundry_room_min_sf)
}

pub fn mail_package_room_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    let mailboxes = units as f64 * a.support.mailbox_per_unit;
    let lockers = (mailboxes * a.support.locker_per_mailbox_ratio).ceil();

    let modules_mail = mailboxes / a.support.mailboxes_per_cabinet;
    let modules_locker = lockers / a.support.lockers_per_cabinet;
    let modules = modules_mail.max(modules_locker);

    let room_width_ft =
        (a.support.mail_cabinet_depth_in + a.support.mail_front_clear_depth_in) / 12.0;
    let room_length_ft = modules * a.support.mail_cabinet_width_in / 12.0;
    let raw = room_width_ft * room_length_ft;

    clamp(
        raw,
        a.support.mail_room_sf_per_du_min * units as f64,
        a.support.mail_room_sf_per_du_max * units as f64,
    )
}

fn interpolated_count_from_units(units: u32, units_1: f64, qty_1: f64, units_2: f64, qty_2: f64) -> u32 {
    let qty = lerp(units as f64, units_1, qty_1, units_2, qty_2)
        .clamp(qty_1.min(qty_2), qty_1.max(qty_2))
        .round();
    qty.max(0.0) as u32
}

pub fn general_storage_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    let qty = interpolated_count_from_units(
        units,
        a.support.general_storage_units_1,
        a.support.general_storage_qty_1,
        a.support.general_storage_units_2,
        a.support.general_storage_qty_2,
    );
    qty as f64 * a.support.general_storage_room_w_ft * a.support.general_storage_room_d_ft
}

pub fn janitor_closet_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    let qty = interpolated_count_from_units(
        units,
        a.support.janitor_units_1,
        a.support.janitor_qty_1,
        a.support.janitor_units_2,
        a.support.janitor_qty_2,
    );
    qty as f64 * a.support.janitor_room_w_ft * a.support.janitor_room_d_ft
}

pub fn parcel_storage_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    let qty = interpolated_count_from_units(
        units,
        a.support.parcel_storage_units_1,
        a.support.parcel_storage_qty_1,
        a.support.parcel_storage_units_2,
        a.support.parcel_storage_qty_2,
    );
    qty as f64 * a.support.parcel_storage_room_w_ft * a.support.parcel_storage_room_d_ft
}

pub fn cold_storage_delivery_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    let qty = interpolated_count_from_units(
        units,
        a.support.cold_storage_units_1,
        a.support.cold_storage_qty_1,
        a.support.cold_storage_units_2,
        a.support.cold_storage_qty_2,
    );
    qty as f64 * a.support.cold_storage_room_w_ft * a.support.cold_storage_room_d_ft
}

pub fn leasing_office_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    let qty = interpolated_count_from_units(
        units,
        a.support.leasing_office_units_1,
        a.support.leasing_office_qty_1,
        a.support.leasing_office_units_2,
        a.support.leasing_office_qty_2,
    );
    qty as f64 * a.support.leasing_office_room_w_ft * a.support.leasing_office_room_d_ft
}

pub fn support_uses_affordable_profile(a: &AssumptionPack) -> bool {
    a.preliminary_area.default_affordable_support_profile
}

pub fn manager_office_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    if a.support.manager_office_affordable_only && !support_uses_affordable_profile(a) {
        return 0.0;
    }
    let qty = interpolated_count_from_units(
        units,
        a.support.manager_office_units_1,
        a.support.manager_office_qty_1,
        a.support.manager_office_units_2,
        a.support.manager_office_qty_2,
    );
    qty as f64 * a.support.manager_office_room_w_ft * a.support.manager_office_room_d_ft
}

pub fn cctv_camera_count(units: u32) -> f64 {
    let units_f = units as f64;
    if units == 0 {
        0.0
    } else if units < 150 {
        4.0 * units_f / 15.0
    } else if units < 400 {
        0.28 * units_f + 8.0
    } else {
        0.325 * units_f - 10.0
    }
}

pub fn cctv_room_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    if !a.support.cctv_room_enabled || units == 0 {
        return 0.0;
    }
    let units_f = units as f64;
    let cameras = cctv_camera_count(units);
    if units < 150 {
        a.support.cctv_room_min_sf
    } else if units < 400 {
        (3.0 * cameras + 690.0) / 7.0
    } else if units < 800 {
        (6.0 * cameras + 2400.0) / 13.0
    } else {
        let operators = (cameras / 100.0).floor();
        let racks = (cameras / 40.0).floor();
        operators * 110.0 + racks * 25.0 + 40.0
    }
    .max(a.support.cctv_room_min_sf)
    .min(if units_f < 150.0 { a.support.cctv_room_min_sf } else { f64::INFINITY })
}

pub fn staff_break_room_count(units: u32, a: &AssumptionPack) -> u32 {
    if !a.support.staff_break_room_enabled || units == 0 {
        return 0;
    }
    let units_1 = a.support.staff_break_room_units_1;
    let units_2 = a.support.staff_break_room_units_2;
    let qty_1 = a.support.staff_break_room_qty_1;
    let qty_2 = a.support.staff_break_room_qty_2;
    let t = if (units_2 - units_1).abs() <= EPS {
        0.0
    } else {
        ((units as f64 - units_1) / (units_2 - units_1)).clamp(0.0, 1.0)
    };
    (qty_1 + (qty_2 - qty_1) * t).round().max(0.0) as u32
}

pub fn staff_break_room_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    staff_break_room_count(units, a) as f64 * a.support.staff_break_room_area_sf
}

pub fn staff_locker_showers_area_sf(staff_count: u32, a: &AssumptionPack) -> f64 {
    if !a.support.staff_locker_showers_enabled || staff_count == 0 {
        return 0.0;
    }
    let per_staff_area = (a.support.staff_locker_showers_base_sf
        * (1.0 + a.support.staff_locker_showers_circulation_ratio))
        .round();
    staff_count as f64 * per_staff_area
}

fn staff_restroom_fixture_group_1_count(staff_count: u32) -> u32 {
    match staff_count {
        0 => 0,
        1..=15 => 1,
        16..=30 => 2,
        31..=50 => 3,
        51..=100 => 4,
        101..=200 => 8,
        201..=400 => 11,
        _ => 11 + (((staff_count - 400) as f64) / 125.0).round().max(0.0) as u32,
    }
}

fn staff_restroom_fixture_group_2_count(staff_count: u32) -> u32 {
    match staff_count {
        0 => 0,
        1..=100 => 1,
        101..=200 => 2,
        201..=400 => 3,
        401..=600 => 4,
        _ => 4 + (((staff_count - 600) as f64) / 300.0).round().max(0.0) as u32,
    }
}

fn staff_restroom_fixture_group_3_count(staff_count: u32) -> u32 {
    match staff_count {
        0 => 0,
        1..=50 => 1,
        51..=100 => 2,
        101..=150 => 3,
        151..=200 => 4,
        201..=300 => 5,
        301..=400 => 6,
        _ => 6 + (((staff_count - 400) as f64) / 200.0).round().max(0.0) as u32,
    }
}

fn staff_restroom_fixture_counts(
    staff_count: u32,
    has_leasing_office: bool,
    a: &AssumptionPack,
) -> (u32, u32, u32) {
    if !a.support.staff_restroom_enabled || staff_count == 0 || !has_leasing_office {
        return (0, 0, 0);
    }
    (
        staff_restroom_fixture_group_1_count(staff_count),
        staff_restroom_fixture_group_2_count(staff_count),
        staff_restroom_fixture_group_3_count(staff_count),
    )
}

pub fn staff_restroom_area_sf(
    staff_count: u32,
    has_leasing_office: bool,
    a: &AssumptionPack,
) -> f64 {
    let (wc_count, urinal_count, lav_count) =
        staff_restroom_fixture_counts(staff_count, has_leasing_office, a);
    if wc_count == 0 && urinal_count == 0 && lav_count == 0 {
        return 0.0;
    }
    let fixture_area = wc_count as f64 * a.support.staff_restroom_fixture_1_area_sf
        + urinal_count as f64 * a.support.staff_restroom_fixture_2_area_sf
        + lav_count as f64 * a.support.staff_restroom_fixture_3_area_sf;
    (fixture_area * (1.0 + a.support.staff_restroom_circulation_ratio))
        .max(a.support.staff_restroom_min_sf)
}

pub fn entry_lobby_area_sf(stories: u32, units: u32, a: &AssumptionPack) -> f64 {
    let min_per_unit = if stories <= a.vertical.mid_rise_max_stories {
        a.corridor_core.entry_lobby_sf_per_unit_low_mid
    } else {
        a.corridor_core.entry_lobby_sf_per_unit_high_tower
    };
    let min_area = min_per_unit * units as f64;
    let interp = lerp(
        units as f64,
        a.corridor_core.entry_lobby_interp_units_1,
        a.corridor_core.entry_lobby_interp_sf_1,
        a.corridor_core.entry_lobby_interp_units_2,
        a.corridor_core.entry_lobby_interp_sf_2,
    );
    min_area.max(interp)
}

pub fn entry_wind_lobby_count(stories: u32, units: u32, a: &AssumptionPack) -> f64 {
    // Workbook gating also depends on a residential-unit-type field that is not yet surfaced
    // in the core input. Until that symbol exists, use the affordable-support profile as the
    // safe proxy and keep only the explicit stories / DU enable thresholds.
    if a.corridor_core.entry_wind_lobby_disable_for_affordable_profile
        && support_uses_affordable_profile(a)
    {
        return 0.0;
    }
    if stories <= a.corridor_core.entry_wind_lobby_enable_min_stories_exclusive
        && units <= a.corridor_core.entry_wind_lobby_enable_min_units_exclusive
    {
        return 0.0;
    }
    lerp(
        units as f64,
        a.corridor_core.entry_wind_lobby_units_1,
        a.corridor_core.entry_wind_lobby_qty_1,
        a.corridor_core.entry_wind_lobby_units_2,
        a.corridor_core.entry_wind_lobby_qty_2,
    )
    .max(0.0)
}

pub fn entry_wind_lobby_area_sf(stories: u32, units: u32, a: &AssumptionPack) -> f64 {
    entry_wind_lobby_count(stories, units, a)
        * a.corridor_core.entry_wind_lobby_room_w_ft
        * a.corridor_core.entry_wind_lobby_room_d_ft
}

pub fn amenity_multiplier(strategy: AmenityStrategy, a: &AssumptionPack) -> f64 {
    match strategy {
        AmenityStrategy::MinCode => a.amenity.multiplier_min_code,
        AmenityStrategy::Balanced => a.amenity.multiplier_balanced,
        AmenityStrategy::Premium => a.amenity.multiplier_premium,
        AmenityStrategy::UserSelected => a.amenity.multiplier_user_selected,
    }
}

pub fn indoor_amenity_target_sf(input: &NormalizedInput, units: u32, a: &AssumptionPack) -> f64 {
    let base = amenity_multiplier(input.amenities.strategy, a)
        * a.amenity.indoor_min_sf_per_du
        * units as f64;
    input.amenities.indoor_target_sf.unwrap_or(base).max(base)
}

pub fn outdoor_amenity_target_sf(input: &NormalizedInput, units: u32, a: &AssumptionPack) -> f64 {
    let base = amenity_multiplier(input.amenities.strategy, a)
        * a.amenity.outdoor_min_sf_per_du
        * units as f64;
    input.amenities.outdoor_target_sf.unwrap_or(base).max(base)
}

fn normalize_amenity_key(name: &str) -> String {
    let mut out = String::new();
    let mut last_was_sep = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_sep = false;
        } else if !out.is_empty() && !last_was_sep {
            out.push('_');
            last_was_sep = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

fn canonical_amenity_catalog_key(name: &str) -> String {
    match normalize_amenity_key(name).as_str() {
        "art" => "art_studio".to_string(),
        "community_multi_purpose_room" => "club_room".to_string(),
        "children_play_area" | "childrens_play_area" => "children_s_play_area".to_string(),
        "fitness_room" => "fitness".to_string(),
        "business_center_coworking" => "cowork".to_string(),
        "dog_washing_room" => "pet_spa".to_string(),
        "bar_cafe_nook" => "sky_lounge".to_string(),
        "concierge_desk" => "concierge".to_string(),
        "conference" => "conference_room".to_string(),
        "dining" => "dining_room".to_string(),
        "game" => "game_room".to_string(),
        "golf_simulator" => "golf_simulator_room".to_string(),
        "library" | "reading_lounge" => "library_reading_lounge".to_string(),
        "massage_room" | "massage_treatment_room" => "massage".to_string(),
        "podcast" => "podcast_room".to_string(),
        "recording" => "recording_room".to_string(),
        "shower_changing" | "changing_room" => "shower_changing_room".to_string(),
        "spa_wellness" | "spa_wellness_center" | "wellness_center" => "spa".to_string(),
        "sauna_steam" | "sauna_steam_room" | "steam_room" => "sauna".to_string(),
        "spin" => "spin_studio".to_string(),
        "screening_room" | "theater_screening_room" => "theater".to_string(),
        "yoga" | "pilates" => "yoga_pilates_room".to_string(),
        other => other.to_string(),
    }
}

fn indoor_amenity_include_keys(input: &NormalizedInput) -> BTreeSet<String> {
    let excludes = input
        .amenities
        .exclude
        .iter()
        .map(|name| canonical_amenity_catalog_key(name))
        .collect::<BTreeSet<_>>();
    input.amenities
        .include
        .iter()
        .map(|name| canonical_amenity_catalog_key(name))
        .filter(|name| !name.is_empty() && !excludes.contains(name))
        .collect::<BTreeSet<_>>()
}

fn amenity_catalog_area_sf(name: &str, a: &AssumptionPack) -> Option<f64> {
    let needle = canonical_amenity_catalog_key(name);
    a.amenity
        .catalog
        .iter()
        .find(|entry| entry.indoor && normalize_amenity_key(&entry.name) == needle)
        .map(|entry| entry.area_sf)
}

#[derive(Debug, Clone, Copy)]
struct WorkbookIndoorAmenityProfile {
    space_name: &'static str,
    min_area_sf: f64,
    score_cost: f64,
    score_effect_pct: f64,
}

fn workbook_indoor_amenity_profile_for_include_key(
    name: &str,
) -> Option<WorkbookIndoorAmenityProfile> {
    match canonical_amenity_catalog_key(name).as_str() {
        "art_studio" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Art Studio",
            min_area_sf: 250.0,
            score_cost: 230.0,
            score_effect_pct: 0.0,
        }),
        "sky_lounge" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Bar / Café Nook",
            min_area_sf: 120.0,
            score_cost: 290.0,
            score_effect_pct: 3.2,
        }),
        "cowork" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Business Center / Coworking",
            min_area_sf: 250.0,
            score_cost: 260.0,
            score_effect_pct: 2.5,
        }),
        "children_s_play_area" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Children’s Play Area",
            min_area_sf: 350.0,
            score_cost: 260.0,
            score_effect_pct: 0.0,
        }),
        "club_room" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Community / Multi-Purpose Room",
            min_area_sf: 700.0,
            score_cost: 240.0,
            score_effect_pct: 2.8,
        }),
        "concierge" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Concierge Desk",
            min_area_sf: 80.0,
            score_cost: 230.0,
            score_effect_pct: 4.2,
        }),
        "conference_room" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Conference Room",
            min_area_sf: 150.0,
            score_cost: 230.0,
            score_effect_pct: 1.2,
        }),
        "dining_room" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Dining Room",
            min_area_sf: 400.0,
            score_cost: 290.0,
            score_effect_pct: 1.5,
        }),
        "pet_spa" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Dog Washing Room",
            min_area_sf: 120.0,
            score_cost: 220.0,
            score_effect_pct: 0.0,
        }),
        "fitness" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Fitness Room",
            min_area_sf: 600.0,
            score_cost: 271.0,
            score_effect_pct: 4.5,
        }),
        "game_room" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Game Room",
            min_area_sf: 300.0,
            score_cost: 235.0,
            score_effect_pct: 0.5,
        }),
        "golf_simulator_room" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Golf Simulator Room",
            min_area_sf: 350.0,
            score_cost: 365.0,
            score_effect_pct: 3.8,
        }),
        "library_reading_lounge" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Library / Reading Lounge",
            min_area_sf: 250.0,
            score_cost: 260.0,
            score_effect_pct: 0.8,
        }),
        "massage" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Massage / Treatment Room",
            min_area_sf: 100.0,
            score_cost: 321.0,
            score_effect_pct: 2.8,
        }),
        "podcast_room" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Podcast Room",
            min_area_sf: 80.0,
            score_cost: 275.0,
            score_effect_pct: 0.0,
        }),
        "recording_room" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Recording Room",
            min_area_sf: 120.0,
            score_cost: 275.0,
            score_effect_pct: 0.0,
        }),
        "sauna" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Sauna / Steam Room",
            min_area_sf: 120.0,
            score_cost: 225.0,
            score_effect_pct: 5.0,
        }),
        "shower_changing_room" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Shower / Changing Room",
            min_area_sf: 200.0,
            score_cost: 378.0,
            score_effect_pct: 0.0,
        }),
        "spa" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Spa / Wellness Center",
            min_area_sf: 400.0,
            score_cost: 265.0,
            score_effect_pct: 6.0,
        }),
        "spin_studio" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Spin Studio",
            min_area_sf: 400.0,
            score_cost: 321.0,
            score_effect_pct: 1.8,
        }),
        "theater" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Theater / Screening Room",
            min_area_sf: 250.0,
            score_cost: 210.0,
            score_effect_pct: 3.5,
        }),
        "yoga_pilates_room" => Some(WorkbookIndoorAmenityProfile {
            space_name: "Yoga / Pilates Room",
            min_area_sf: 400.0,
            score_cost: 315.0,
            score_effect_pct: 2.0,
        }),
        _ => None,
    }
}

fn indoor_amenity_program_space_name_for_include_key(name: &str) -> Option<&'static str> {
    workbook_indoor_amenity_profile_for_include_key(name).map(|profile| profile.space_name)
}

fn fallback_indoor_amenity_detailed_program_areas_from_catalog(
    include_keys: &BTreeSet<String>,
    indoor_amenity_sf: f64,
    a: &AssumptionPack,
) -> Vec<(String, f64)> {
    let mut detail_rows = include_keys
        .iter()
        .filter_map(|key| {
            indoor_amenity_program_space_name_for_include_key(key).and_then(|space_name| {
                amenity_catalog_area_sf(key, a)
                    .or_else(|| {
                        workbook_indoor_amenity_profile_for_include_key(key)
                            .map(|profile| profile.min_area_sf)
                    })
                    .filter(|area_sf| *area_sf > EPS)
                    .map(|area_sf| (space_name.to_string(), area_sf))
            })
        })
        .collect::<Vec<_>>();
    if detail_rows.is_empty() {
        return Vec::new();
    }
    let total_catalog_area = detail_rows.iter().map(|(_, area_sf)| *area_sf).sum::<f64>();
    let scale = if total_catalog_area > indoor_amenity_sf {
        indoor_amenity_sf / total_catalog_area.max(EPS)
    } else {
        1.0
    };
    let mut aggregated = BTreeMap::<String, f64>::new();
    for (space_name, area_sf) in detail_rows.drain(..) {
        *aggregated.entry(space_name).or_insert(0.0) += area_sf * scale;
    }
    aggregated
        .into_iter()
        .filter(|(_, area_sf)| *area_sf > EPS)
        .collect()
}

fn workbook_indoor_amenity_detailed_program_areas(
    include_keys: &BTreeSet<String>,
    indoor_amenity_sf: f64,
    a: &AssumptionPack,
) -> Vec<(String, f64)> {
    if indoor_amenity_sf <= EPS {
        return Vec::new();
    }
    let manual_include_keys = include_keys
        .iter()
        .filter(|key| {
            workbook_indoor_amenity_profile_for_include_key(key)
                .map(|profile| profile.score_effect_pct <= EPS || profile.score_cost <= EPS)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    let manual_rows =
        fallback_indoor_amenity_detailed_program_areas_from_catalog(&manual_include_keys, indoor_amenity_sf, a);
    let manual_area_sf = manual_rows.iter().map(|(_, area_sf)| *area_sf).sum::<f64>();
    let remaining_amenity_sf = (indoor_amenity_sf - manual_area_sf).max(0.0);
    if remaining_amenity_sf <= EPS {
        return manual_rows;
    }
    let profiles = include_keys
        .iter()
        .filter_map(|key| workbook_indoor_amenity_profile_for_include_key(key))
        .filter(|profile| profile.score_effect_pct > EPS && profile.score_cost > EPS)
        .collect::<Vec<_>>();
    if profiles.is_empty() {
        return manual_rows;
    }
    let raw_scores = profiles
        .iter()
        .map(|profile| profile.score_effect_pct / profile.score_cost.max(EPS))
        .collect::<Vec<_>>();
    let sum_score_1 = raw_scores.iter().sum::<f64>();
    if sum_score_1 <= EPS {
        return manual_rows;
    }
    let mask = profiles
        .iter()
        .zip(raw_scores.iter())
        .map(|(profile, raw_score)| {
            let sf_alloc_1 = raw_score / sum_score_1 * remaining_amenity_sf;
            let qty_raw_1 = sf_alloc_1 / profile.min_area_sf.max(EPS);
            if qty_raw_1 >= 0.5 { 1.0 } else { 0.0 }
        })
        .collect::<Vec<_>>();
    let sum_score_2 = raw_scores
        .iter()
        .zip(mask.iter())
        .map(|(raw_score, mask_value)| raw_score * mask_value)
        .sum::<f64>();
    if sum_score_2 <= EPS {
        return manual_rows;
    }
    let mut aggregated = manual_rows
        .into_iter()
        .collect::<BTreeMap<String, f64>>();
    for (space_name, area_sf) in profiles
        .iter()
        .zip(raw_scores.iter())
        .zip(mask.iter())
        .filter_map(|((profile, raw_score), mask_value)| {
            if *mask_value <= EPS {
                return None;
            }
            let sf_final_1 = raw_score / sum_score_2 * remaining_amenity_sf;
            let qty_final = (sf_final_1 / profile.min_area_sf.max(EPS)).floor();
            if qty_final < 1.0 || sf_final_1 <= EPS {
                None
            } else {
                Some((profile.space_name.to_string(), sf_final_1))
            }
        })
    {
        *aggregated.entry(space_name).or_insert(0.0) += area_sf;
    }
    aggregated
        .into_iter()
        .filter(|(_, area_sf)| *area_sf > EPS)
        .collect()
}

pub fn indoor_amenity_detailed_program_areas(
    input: &NormalizedInput,
    indoor_amenity_sf: f64,
    a: &AssumptionPack,
) -> Vec<(String, f64)> {
    let include_keys = indoor_amenity_include_keys(input);
    let workbook_rows =
        workbook_indoor_amenity_detailed_program_areas(&include_keys, indoor_amenity_sf, a);
    if !workbook_rows.is_empty() {
        return workbook_rows;
    }
    fallback_indoor_amenity_detailed_program_areas_from_catalog(&include_keys, indoor_amenity_sf, a)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResidentRestroomOccClass {
    A1,
    A3,
    M,
    B,
}

fn resident_restroom_occ_profile_for_amenity(
    name: &str,
) -> Option<(ResidentRestroomOccClass, f64)> {
    match canonical_amenity_catalog_key(name).as_str() {
        "club_room" => Some((ResidentRestroomOccClass::A3, 15.0)),
        "fitness" => Some((ResidentRestroomOccClass::A3, 50.0)),
        "cowork" => Some((ResidentRestroomOccClass::B, 150.0)),
        "pet_spa" => Some((ResidentRestroomOccClass::B, 200.0)),
        "sky_lounge" => Some((ResidentRestroomOccClass::A3, 15.0)),
        "concierge" => Some((ResidentRestroomOccClass::B, 150.0)),
        "massage" => Some((ResidentRestroomOccClass::B, 100.0)),
        "spa" => Some((ResidentRestroomOccClass::A3, 50.0)),
        "sauna" => Some((ResidentRestroomOccClass::A3, 50.0)),
        "theater" => Some((ResidentRestroomOccClass::A1, 7.0)),
        _ => None,
    }
}

fn resident_restroom_occ_profile_for_space_name(
    space_name: &str,
) -> Option<(ResidentRestroomOccClass, f64)> {
    match space_name.to_ascii_lowercase().as_str() {
        "community / multi-purpose room" => Some((ResidentRestroomOccClass::A3, 15.0)),
        "fitness room" => Some((ResidentRestroomOccClass::A3, 50.0)),
        "business center / coworking" => Some((ResidentRestroomOccClass::B, 150.0)),
        "dog washing room" => Some((ResidentRestroomOccClass::B, 200.0)),
        "bar / café nook" | "bar / cafe nook" => Some((ResidentRestroomOccClass::A3, 15.0)),
        "concierge desk" => Some((ResidentRestroomOccClass::B, 150.0)),
        "massage / treatment room" => Some((ResidentRestroomOccClass::B, 100.0)),
        "spa / wellness center" => Some((ResidentRestroomOccClass::A3, 50.0)),
        "sauna / steam room" => Some((ResidentRestroomOccClass::A3, 50.0)),
        "theater / screening room" => Some((ResidentRestroomOccClass::A1, 7.0)),
        _ => None,
    }
}

fn resident_restroom_male_wc_count(class: ResidentRestroomOccClass, occ: u32) -> u32 {
    if occ == 0 {
        return 0;
    }
    match class {
        ResidentRestroomOccClass::A1
        | ResidentRestroomOccClass::A3
        | ResidentRestroomOccClass::M => {
            if occ <= 100 {
                1
            } else if occ <= 200 {
                2
            } else if occ <= 400 {
                3
            } else {
                3 + (((occ - 400) as f64) / 500.0).round().max(0.0) as u32
            }
        }
        ResidentRestroomOccClass::B => {
            if occ <= 50 {
                1
            } else if occ <= 100 {
                2
            } else if occ <= 200 {
                3
            } else if occ <= 400 {
                4
            } else {
                4 + (((occ - 400) as f64) / 500.0).round().max(0.0) as u32
            }
        }
    }
}

fn resident_restroom_male_urinal_count(class: ResidentRestroomOccClass, occ: u32) -> u32 {
    if occ == 0 {
        return 0;
    }
    match class {
        ResidentRestroomOccClass::A1 => {
            if occ <= 200 {
                1
            } else if occ <= 300 {
                2
            } else if occ <= 400 {
                3
            } else if occ <= 600 {
                4
            } else {
                4 + (((occ - 600) as f64) / 300.0).round().max(0.0) as u32
            }
        }
        ResidentRestroomOccClass::A3 | ResidentRestroomOccClass::B => {
            if occ <= 100 {
                1
            } else if occ <= 200 {
                2
            } else if occ <= 400 {
                3
            } else if occ <= 600 {
                4
            } else {
                4 + (((occ - 600) as f64) / 300.0).round().max(0.0) as u32
            }
        }
        ResidentRestroomOccClass::M => {
            if occ <= 200 {
                0
            } else if occ <= 400 {
                1
            } else {
                1 + (((occ - 400) as f64) / 500.0).round().max(0.0) as u32
            }
        }
    }
}

fn resident_restroom_male_lav_count(class: ResidentRestroomOccClass, occ: u32) -> u32 {
    if occ == 0 {
        return 0;
    }
    match class {
        ResidentRestroomOccClass::A1 | ResidentRestroomOccClass::A3 => {
            if occ <= 200 {
                1
            } else if occ <= 400 {
                2
            } else if occ <= 600 {
                3
            } else if occ <= 750 {
                4
            } else {
                4 + (((occ - 750) as f64) / 250.0).round().max(0.0) as u32
            }
        }
        ResidentRestroomOccClass::M => {
            if occ <= 200 {
                1
            } else if occ <= 400 {
                2
            } else {
                2 + (((occ - 400) as f64) / 500.0).round().max(0.0) as u32
            }
        }
        ResidentRestroomOccClass::B => {
            if occ <= 75 {
                1
            } else if occ <= 150 {
                2
            } else if occ <= 200 {
                3
            } else if occ <= 300 {
                4
            } else if occ <= 400 {
                5
            } else {
                5 + (((occ - 400) as f64) / 250.0).round().max(0.0) as u32
            }
        }
    }
}

fn resident_restroom_female_wc_count(class: ResidentRestroomOccClass, occ: u32) -> u32 {
    if occ == 0 {
        return 0;
    }
    match class {
        ResidentRestroomOccClass::A1 | ResidentRestroomOccClass::A3 => {
            if occ <= 25 {
                1
            } else if occ <= 50 {
                2
            } else if occ <= 100 {
                3
            } else if occ <= 200 {
                4
            } else if occ <= 300 {
                6
            } else if occ <= 400 {
                8
            } else {
                8 + (((occ - 400) as f64) / 125.0).round().max(0.0) as u32
            }
        }
        ResidentRestroomOccClass::M => {
            if occ <= 100 {
                1
            } else if occ <= 200 {
                2
            } else if occ <= 300 {
                4
            } else if occ <= 400 {
                6
            } else {
                6 + (((occ - 400) as f64) / 200.0).round().max(0.0) as u32
            }
        }
        ResidentRestroomOccClass::B => {
            if occ <= 15 {
                1
            } else if occ <= 30 {
                2
            } else if occ <= 50 {
                3
            } else if occ <= 100 {
                4
            } else if occ <= 200 {
                8
            } else if occ <= 400 {
                11
            } else {
                11 + (((occ - 400) as f64) / 125.0).round().max(0.0) as u32
            }
        }
    }
}

fn resident_restroom_female_lav_count(class: ResidentRestroomOccClass, occ: u32) -> u32 {
    if occ == 0 {
        return 0;
    }
    match class {
        ResidentRestroomOccClass::A1 | ResidentRestroomOccClass::A3 => {
            if occ <= 100 {
                1
            } else if occ <= 200 {
                2
            } else if occ <= 300 {
                4
            } else if occ <= 500 {
                5
            } else if occ <= 750 {
                6
            } else {
                6 + (((occ - 750) as f64) / 200.0).round().max(0.0) as u32
            }
        }
        ResidentRestroomOccClass::M => {
            if occ <= 200 {
                1
            } else if occ <= 300 {
                2
            } else if occ <= 400 {
                3
            } else {
                3 + (((occ - 400) as f64) / 400.0).round().max(0.0) as u32
            }
        }
        ResidentRestroomOccClass::B => {
            if occ <= 50 {
                1
            } else if occ <= 100 {
                2
            } else if occ <= 150 {
                3
            } else if occ <= 200 {
                4
            } else if occ <= 300 {
                5
            } else if occ <= 400 {
                6
            } else {
                6 + (((occ - 400) as f64) / 200.0).round().max(0.0) as u32
            }
        }
    }
}

pub fn resident_restroom_area_sf(
    input: &NormalizedInput,
    indoor_amenity_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    let (male_wc, male_urinal, male_lav, female_wc, female_lav) =
        resident_restroom_fixture_counts(input, indoor_amenity_sf, a);
    if male_wc == 0
        && male_urinal == 0
        && male_lav == 0
        && female_wc == 0
        && female_lav == 0
    {
        return 0.0;
    }
    let male_area = (male_wc as f64 * a.amenity.resident_restroom_wc_area_sf
        + male_urinal as f64 * a.amenity.resident_restroom_urinal_area_sf
        + male_lav as f64 * a.amenity.resident_restroom_lavatory_area_sf)
        * (1.0 + a.amenity.resident_restroom_circulation_ratio);
    let female_area = (female_wc as f64 * a.amenity.resident_restroom_wc_area_sf
        + female_lav as f64 * a.amenity.resident_restroom_lavatory_area_sf)
        * (1.0 + a.amenity.resident_restroom_circulation_ratio);

    male_area.max(a.amenity.resident_restroom_min_sf)
        + female_area.max(a.amenity.resident_restroom_min_sf)
}

pub fn commercial_kitchen_shaft_area_sf(
    input: &NormalizedInput,
    units: u32,
    total_stories: u32,
    a: &AssumptionPack,
) -> f64 {
    let include_keys = indoor_amenity_include_keys(input);
    if !include_keys.contains("commercial_kitchen") || units == 0 || total_stories == 0 {
        return 0.0;
    }
    let units_1 = a.boh.commercial_kitchen_shaft_range_qty_units_1;
    let units_2 = a.boh.commercial_kitchen_shaft_range_qty_units_2;
    let qty_1 = a.boh.commercial_kitchen_shaft_range_qty_1;
    let qty_2 = a.boh.commercial_kitchen_shaft_range_qty_2;
    let t = if (units_2 - units_1).abs() <= EPS {
        0.0
    } else {
        ((units as f64 - units_1) / (units_2 - units_1)).clamp(0.0, 1.0)
    };
    let range_qty = (qty_1 + (qty_2 - qty_1) * t).round().max(1.0);
    let hood_length_ft = range_qty * a.boh.commercial_kitchen_shaft_range_width_ft
        + a.boh.commercial_kitchen_shaft_hood_length_offset_ft;
    let exhaust_rate_cfm_per_ft = if a.boh.commercial_kitchen_shaft_use_hood_ul710 {
        a.boh.commercial_kitchen_shaft_ul710_exhaust_rate_cfm_per_ft
    } else if a.boh.commercial_kitchen_shaft_range_type_electric {
        a.boh.commercial_kitchen_shaft_electric_exhaust_rate_cfm_per_ft
    } else {
        a.boh.commercial_kitchen_shaft_gas_exhaust_rate_cfm_per_ft
    };
    let exhaust_air_cfm = exhaust_rate_cfm_per_ft
        * hood_length_ft
        * a.boh.commercial_kitchen_shaft_diversity_factor
        * (1.0 + a.boh.commercial_kitchen_shaft_future_expansion_ratio);
    let grease_duct_area_sf = exhaust_air_cfm / a.boh.commercial_kitchen_shaft_exhaust_velocity_fpm.max(EPS);
    let grease_diameter_in = ((grease_duct_area_sf * 4.0 / std::f64::consts::PI)
        .sqrt()
        * 12.0)
        .ceil()
        + a.boh.commercial_kitchen_shaft_round_duct_upsize_in;
    let makeup_diameter_in = ((grease_duct_area_sf
        * a.boh.commercial_kitchen_shaft_diversity_factor
        * 4.0
        / std::f64::consts::PI)
        .sqrt()
        * 12.0)
        .ceil()
        + a.boh.commercial_kitchen_shaft_round_duct_upsize_in;
    let clearance_in = a.boh.commercial_kitchen_shaft_clearance_in;
    let grease_shaft_area_sf = (grease_diameter_in + 2.0 * clearance_in).powi(2) / 144.0;
    let makeup_shaft_area_sf = (makeup_diameter_in + 2.0 * clearance_in).powi(2) / 144.0;
    let waste_vent_area_sf = (a.boh.commercial_kitchen_shaft_waste_vent_width_in
        + 2.0 * clearance_in)
        * (a.boh.commercial_kitchen_shaft_waste_vent_width_in * 2.0 + 2.0 * clearance_in)
        / 144.0;
    let area_per_floor_sf = grease_shaft_area_sf
        + makeup_shaft_area_sf
        + waste_vent_area_sf
        + a.boh.commercial_kitchen_shaft_other_system_area_sf;
    area_per_floor_sf * total_stories as f64
}

fn resident_restroom_fixture_counts(
    input: &NormalizedInput,
    indoor_amenity_sf: f64,
    a: &AssumptionPack,
) -> (u32, u32, u32, u32, u32) {
    let detailed_rows = indoor_amenity_detailed_program_areas(input, indoor_amenity_sf, a);
    if !detailed_rows
        .iter()
        .any(|(space_name, area_sf)| space_name == "Community / Multi-Purpose Room" && *area_sf > EPS)
    {
        return (0, 0, 0, 0, 0);
    }

    let mut amenity_areas = detailed_rows
        .into_iter()
        .filter_map(|(space_name, area_sf)| {
            resident_restroom_occ_profile_for_space_name(&space_name)
                .map(|(class, olf)| (class, olf, area_sf))
        })
        .collect::<Vec<_>>();
    if amenity_areas.is_empty() {
        return (0, 0, 0, 0, 0);
    }

    let mut male_wc = 0u32;
    let mut male_urinal = 0u32;
    let mut male_lav = 0u32;
    let mut female_wc = 0u32;
    let mut female_lav = 0u32;

    for (class, olf, area_sf) in amenity_areas.drain(..) {
        let occupant_count = (area_sf / olf.max(EPS)).ceil().max(0.0) as u32;
        let male_occ = ((occupant_count as f64) * 0.5).ceil() as u32;
        let female_occ = ((occupant_count as f64) * 0.5).ceil() as u32;
        male_wc += resident_restroom_male_wc_count(class, male_occ);
        male_urinal += resident_restroom_male_urinal_count(class, male_occ);
        male_lav += resident_restroom_male_lav_count(class, male_occ);
        female_wc += resident_restroom_female_wc_count(class, female_occ);
        female_lav += resident_restroom_female_lav_count(class, female_occ);
    }

    (male_wc, male_urinal, male_lav, female_wc, female_lav)
}

pub fn domestic_public_plumbing_fixture_counts(
    input: &NormalizedInput,
    indoor_amenity_sf: f64,
    staff_count: u32,
    has_leasing_office: bool,
    a: &AssumptionPack,
) -> (u32, u32, u32) {
    let (staff_wc, staff_urinal, staff_lav) =
        staff_restroom_fixture_counts(staff_count, has_leasing_office, a);
    let (resident_male_wc, resident_male_urinal, resident_male_lav, resident_female_wc, resident_female_lav) =
        resident_restroom_fixture_counts(input, indoor_amenity_sf, a);
    (
        staff_wc + resident_male_wc + resident_female_wc,
        staff_urinal + resident_male_urinal,
        staff_lav + resident_male_lav + resident_female_lav,
    )
}

pub fn amenity_storage_area_sf(indoor_amenity_sf: f64, a: &AssumptionPack) -> f64 {
    indoor_amenity_sf.max(0.0) * a.amenity.amenity_storage_ratio
}

pub fn outdoor_amenity_circulation_area_sf(outdoor_amenity_sf: f64, a: &AssumptionPack) -> f64 {
    outdoor_amenity_sf.max(0.0) * a.amenity.outdoor_amenity_circulation_ratio
}

pub fn parking_stall_demand(
    counts: [u32; 4],
    retail_area_sf: f64,
    input: &NormalizedInput,
    a: &AssumptionPack,
) -> u32 {
    let res = counts[0] as f64 * a.parking.stalls_per_studio
        + counts[1] as f64 * a.parking.stalls_per_one_bed
        + counts[2] as f64 * a.parking.stalls_per_two_bed
        + counts[3] as f64 * a.parking.stalls_per_three_bed;
    let retail = (retail_area_sf / a.parking.retail_sf_per_stall).ceil();
    let total = res.ceil() + retail;

    match input.constraints.parking_mode {
        ParkingMode::None => 0,
        _ => total.max(0.0) as u32,
    }
}

pub fn parking_gross_sf_per_stall(mode: ParkingMode, a: &AssumptionPack) -> f64 {
    match mode {
        ParkingMode::None => 0.0,
        ParkingMode::Surface => a.parking.gross_sf_per_stall_surface,
        ParkingMode::Podium => a.parking.gross_sf_per_stall_podium,
        ParkingMode::Structured => a.parking.gross_sf_per_stall_structured,
        ParkingMode::Underground => a.parking.gross_sf_per_stall_underground,
        ParkingMode::Mixed => a.parking.gross_sf_per_stall_mixed,
        ParkingMode::Auto => a.parking.gross_sf_per_stall_structured,
    }
}

pub fn loading_zone_count(units: u32) -> u32 {
    (units as f64 / 250.0).ceil().max(0.0) as u32
}

pub fn loading_dock_count(units: u32, a: &AssumptionPack) -> u32 {
    if units == 0 {
        return 0;
    }
    ((units as f64 / a.support.loading_dock_units_per_bay).round())
        .max(1.0) as u32
}

pub fn loading_dock_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    loading_dock_count(units, a) as f64 * a.support.loading_dock_default_sf
}

pub fn support_occupant_count(counts: [u32; 4], a: &AssumptionPack) -> f64 {
    counts[0] as f64 * a.support.trash_occupants_per_studio
        + counts[1] as f64 * a.support.trash_occupants_per_one_bed
        + counts[2] as f64 * a.support.trash_occupants_per_two_bed
        + counts[3] as f64 * a.support.trash_occupants_per_three_bed
}

pub fn support_staff_count(units: u32, a: &AssumptionPack) -> f64 {
    (units as f64 / a.vertical.operations_staff_per_units.max(1.0)).ceil()
}

pub fn coarse_building_occupant_count(counts: [u32; 4], a: &AssumptionPack) -> f64 {
    let units = counts.iter().sum::<u32>();
    counts[0] as f64 * a.boh.building_occupants_per_studio
        + counts[1] as f64 * a.boh.building_occupants_per_one_bedroom
        + counts[2] as f64 * a.boh.building_occupants_per_two_bedroom
        + counts[3] as f64 * a.boh.building_occupants_per_three_bedroom
        + support_staff_count(units, a)
}

pub fn trash_total_volume_cy(counts: [u32; 4], a: &AssumptionPack) -> f64 {
    let units = counts.iter().sum::<u32>();
    (support_occupant_count(counts, a) + support_staff_count(units, a))
        * a.support.trash_volume_cy_per_person_per_week
        / a.support.trash_pickups_per_week.max(1.0)
}

pub fn dumpster_support_area_sf(volume_cy: f64, a: &AssumptionPack) -> f64 {
    if volume_cy <= EPS {
        return 0.0;
    }
    let dumpster_size_cy = if volume_cy.floor().max(0.0) <= 2.0 {
        2.0
    } else if volume_cy.floor().max(0.0) <= 4.0 {
        4.0
    } else if volume_cy.floor().max(0.0) <= 6.0 {
        6.0
    } else {
        8.0
    };
    let dumpster_width_ft = if dumpster_size_cy <= 2.0 {
        3.0
    } else if dumpster_size_cy <= 4.0 {
        3.0
    } else if dumpster_size_cy <= 6.0 {
        5.0
    } else {
        6.0
    };
    let qty = volume_cy / (dumpster_size_cy * a.support.trash_dumpster_fill_factor.max(EPS));
    qty * dumpster_width_ft * a.support.trash_dumpster_length_ft * a.support.trash_clearance_factor
}

pub fn trash_has_chute(total_stories: u32, a: &AssumptionPack) -> bool {
    total_stories >= a.support.trash_chute_min_total_stories
}

pub fn trash_room_count_per_floor(corridor_length_per_floor_ft: f64, a: &AssumptionPack) -> u32 {
    (corridor_length_per_floor_ft / (2.0 * a.support.trash_room_max_distance_ft).max(1.0))
        .ceil()
        .max(1.0) as u32
}

pub fn distributed_trash_room_area_sf(
    counts: [u32; 4],
    corridor_length_per_floor_ft: f64,
    total_stories: u32,
    residential_stories: u32,
    a: &AssumptionPack,
) -> f64 {
    let total_volume = trash_total_volume_cy(counts, a);
    let room_count_per_floor = trash_room_count_per_floor(corridor_length_per_floor_ft, a) as f64;
    if trash_has_chute(total_stories, a) {
        total_stories as f64 * room_count_per_floor * a.support.trash_chute_room_area_sf
    } else {
        let per_room_volume = total_volume
            / (room_count_per_floor * residential_stories.max(1) as f64).max(1.0);
        dumpster_support_area_sf(per_room_volume, a)
            * room_count_per_floor
            * residential_stories.max(1) as f64
    }
}

pub fn typical_trash_room_area_sf(
    counts: [u32; 4],
    corridor_length_per_floor_ft: f64,
    total_stories: u32,
    residential_stories: u32,
    a: &AssumptionPack,
) -> f64 {
    let distributed_area = distributed_trash_room_area_sf(
        counts,
        corridor_length_per_floor_ft,
        total_stories,
        residential_stories,
        a,
    );
    if compactor_room_area_sf(counts, a) > EPS {
        distributed_area
    } else {
        distributed_area + central_trash_room_area_sf(counts, a)
    }
}

pub fn central_trash_room_area_sf(counts: [u32; 4], a: &AssumptionPack) -> f64 {
    let total_volume = trash_total_volume_cy(counts, a);
    let recycle_share = if a.support.trash_recycling_room_enabled {
        a.support.trash_recycling_ratio
    } else {
        0.0
    };
    let compost_share = if a.support.trash_compost_room_enabled {
        a.support.trash_compost_ratio
    } else {
        0.0
    };
    dumpster_support_area_sf(total_volume * (1.0 - recycle_share - compost_share).max(0.0), a)
}

pub fn recycling_room_area_from_counts_sf(counts: [u32; 4], a: &AssumptionPack) -> f64 {
    if !a.support.trash_recycling_room_enabled {
        return 0.0;
    }
    let units = counts.iter().sum::<u32>();
    recycling_room_area_sf(units, a).max(dumpster_support_area_sf(
        trash_total_volume_cy(counts, a) * a.support.trash_recycling_ratio,
        a,
    ))
}

pub fn compost_room_area_sf(counts: [u32; 4], a: &AssumptionPack) -> f64 {
    if !a.support.trash_compost_room_enabled {
        return 0.0;
    }
    dumpster_support_area_sf(
        trash_total_volume_cy(counts, a) * a.support.trash_compost_ratio,
        a,
    )
}

pub fn compactor_room_area_sf(counts: [u32; 4], a: &AssumptionPack) -> f64 {
    let compacted_volume = trash_total_volume_cy(counts, a) / a.support.trash_compaction_ratio.max(1.0);
    let compacted_support_area = dumpster_support_area_sf(compacted_volume, a);
    let compactor_footprint = (a.support.trash_compactor_width_ft
        + a.support.trash_compactor_side_clear_ft)
        * (a.support.trash_compactor_length_ft + a.support.trash_compactor_front_clear_ft);
    let room_area = compactor_footprint + compacted_support_area;
    if central_trash_room_area_sf(counts, a) > room_area {
        room_area
    } else {
        0.0
    }
}

pub fn recycling_room_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    if units <= a.support.recycling_units_small_max {
        a.support.recycling_area_small_sf
    } else if units <= a.support.recycling_units_medium_max {
        a.support.recycling_area_medium_sf
    } else {
        a.support.recycling_area_large_sf
    }
}

pub fn trash_vestibule_area_sf(total_stories: u32, residential_stories: u32, a: &AssumptionPack) -> f64 {
    if total_stories < a.support.trash_vestibule_story_enable_min {
        return 0.0;
    }
    let per_room = if trash_has_chute(total_stories, a) {
        a.support.trash_vestibule_with_chute_sf
    } else {
        a.support.trash_vestibule_without_chute_sf
    };
    residential_stories as f64 * a.support.trash_vestibule_qty_per_res_floor * per_room
}

pub fn parking_control_room_area_sf(
    stalls: u32,
    parking_mode: ParkingMode,
    a: &AssumptionPack,
) -> f64 {
    if stalls == 0
        || stalls <= a.support.parking_control_enable_min_stalls
        || matches!(parking_mode, ParkingMode::None | ParkingMode::Surface)
    {
        return 0.0;
    }
    let stalls_f = stalls as f64;
    if stalls <= a.support.parking_control_piece_1_max_stalls {
        a.support.parking_control_piece_1_slope * stalls_f
            + a.support.parking_control_piece_1_intercept_sf
    } else if stalls <= a.support.parking_control_piece_2_max_stalls {
        (a.support.parking_control_piece_2_slope * stalls_f)
            .max(a.support.parking_control_piece_2_min_sf)
    } else {
        a.support.parking_control_piece_3_slope * stalls_f
            + a.support.parking_control_piece_3_intercept_sf
    }
}

pub fn mpoe_room_count(units: u32, a: &AssumptionPack) -> u32 {
    interpolated_count_from_units(
        units,
        a.boh.mpoe_units_1,
        a.boh.mpoe_qty_1,
        a.boh.mpoe_units_2,
        a.boh.mpoe_qty_2,
    )
}

pub fn mpoe_room_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    mpoe_room_count(units, a) as f64 * a.boh.mpoe_room_w_ft * a.boh.mpoe_room_d_ft
}

pub fn idf_closet_count(units: u32, stories: u32, a: &AssumptionPack) -> u32 {
    if stories <= a.boh.idf_enable_min_stories {
        return 0;
    }
    interpolated_count_from_units(
        units,
        a.boh.idf_units_1,
        a.boh.idf_qty_1,
        a.boh.idf_units_2,
        a.boh.idf_qty_2,
    )
}

pub fn idf_closets_area_sf(units: u32, stories: u32, a: &AssumptionPack) -> f64 {
    let qty = idf_closet_count(units, stories, a);
    let single_area = (a.boh.idf_room_w_ft * a.boh.idf_room_d_ft).min(a.boh.idf_room_max_sf);
    qty as f64 * single_area
}

pub fn das_room_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    a.boh.das_a * units as f64 + a.boh.das_b
}

pub fn main_electrical_room_area_sf(total_kva: f64) -> f64 {
    if total_kva <= 432.0 {
        160.0
    } else if total_kva <= 720.0 {
        210.0
    } else if total_kva <= 1440.0 {
        320.0
    } else if total_kva <= 3324.0 {
        450.0
    } else if total_kva <= 6648.0 {
        800.0
    } else {
        1600.0
    }
}

pub fn electrical_elevator_demand_factor(elevator_count: u32) -> f64 {
    match elevator_count {
        0 => 0.0,
        1 => 1.0,
        2 => 0.95,
        3 => 0.90,
        4 => 0.85,
        5 => 0.82,
        6 => 0.79,
        7 => 0.77,
        8 => 0.75,
        9 => 0.73,
        _ => 0.72,
    }
}

pub fn electrical_dwelling_unit_diversity_factor(units: u32) -> f64 {
    if units < 3 {
        return 1.0;
    }
    match units {
        0..=5 => 0.45,
        6..=7 => 0.44,
        8..=10 => 0.43,
        11 => 0.42,
        12..=13 => 0.41,
        14..=15 => 0.40,
        16..=17 => 0.39,
        18..=20 => 0.38,
        21 => 0.37,
        22..=23 => 0.36,
        24..=25 => 0.35,
        26..=27 => 0.34,
        28..=30 => 0.33,
        31 => 0.32,
        32..=33 => 0.31,
        34..=36 => 0.30,
        37..=38 => 0.29,
        39..=42 => 0.28,
        43..=45 => 0.27,
        46..=50 => 0.26,
        51..=55 => 0.25,
        56..=61 => 0.24,
        _ => 0.23,
    }
}

pub fn coarse_non_residential_electrical_load_kva(
    corridor_area_sf: f64,
    retail_area_sf: f64,
    shaft_stair_area_sf: f64,
    entry_lobby_area_sf: f64,
    indoor_parking_area_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    corridor_area_sf * a.boh.electrical_corridor_w_per_sf / 1000.0
        + retail_area_sf * a.boh.electrical_retail_w_per_sf / 1000.0
        + shaft_stair_area_sf * a.boh.electrical_shaft_stair_w_per_sf / 1000.0
        + entry_lobby_area_sf * a.boh.electrical_entry_lobby_w_per_sf / 1000.0
        + indoor_parking_area_sf * a.boh.electrical_indoor_parking_w_per_sf / 1000.0
}

pub fn coarse_residential_electrical_load_kva(
    units: u32,
    net_residential_area_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    let connected_load_kva = units as f64
        * (a.boh.electrical_dwelling_unit_appliance_connected_kva_per_du
            + a.boh.electrical_dwelling_unit_misc_connected_kva_per_du)
        + net_residential_area_sf * a.boh.electrical_residential_lighting_w_per_sf / 1000.0;
    electrical_dwelling_unit_diversity_factor(units) * connected_load_kva
}

pub fn coarse_total_electrical_load_kva(
    units: u32,
    passenger_elevators: u32,
    freight_elevators: u32,
    net_residential_area_sf: f64,
    corridor_area_sf: f64,
    retail_area_sf: f64,
    shaft_stair_area_sf: f64,
    entry_lobby_area_sf: f64,
    indoor_parking_area_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    let elevator_count = passenger_elevators + freight_elevators;
    let elevator_kva = elevator_count as f64
        * a.boh.electrical_elevator_kva_per_car
        * electrical_elevator_demand_factor(elevator_count);
    elevator_kva
        + a.boh.electrical_motor_constant_kva
        + coarse_non_residential_electrical_load_kva(
            corridor_area_sf,
            retail_area_sf,
            shaft_stair_area_sf,
            entry_lobby_area_sf,
            indoor_parking_area_sf,
            a,
        )
        + coarse_residential_electrical_load_kva(units, net_residential_area_sf, a)
}

pub fn main_electrical_room_project_area_sf(
    total_kva: f64,
    units: u32,
    a: &AssumptionPack,
) -> f64 {
    if units <= a.boh.electrical_main_room_enable_min_units {
        0.0
    } else {
        main_electrical_room_area_sf(total_kva)
    }
}

pub fn electrical_customer_station_indoor_area_sf(total_kva: f64, a: &AssumptionPack) -> f64 {
    if !a.boh.electrical_customer_station_indoor_enabled || total_kva <= EPS {
        return 0.0;
    }
    let tx_class_kva = total_kva.ceil().max(0.0);
    let tx_scale = (tx_class_kva
        / a.boh
            .electrical_customer_station_indoor_transformer_reference_kva
            .max(EPS))
    .sqrt();
    let tx_length_ft = a
        .boh
        .electrical_customer_station_indoor_transformer_base_length_ft
        + a.boh
            .electrical_customer_station_indoor_transformer_length_scale_ft
            * tx_scale;
    let tx_width_ft = a
        .boh
        .electrical_customer_station_indoor_transformer_base_width_ft
        + a.boh
            .electrical_customer_station_indoor_transformer_width_scale_ft
            * tx_scale;
    let tx_envelope_length_ft = tx_length_ft
        + 2.0 * a
            .boh
            .electrical_customer_station_indoor_transformer_side_clear_ft
        + 2.0
            * a.boh
                .electrical_customer_station_indoor_transformer_wall_buffer_ft;
    let tx_envelope_width_ft = tx_width_ft
        + a.boh
            .electrical_customer_station_indoor_transformer_front_clear_ft
        + a.boh
            .electrical_customer_station_indoor_transformer_rear_clear_ft
        + 2.0
            * a.boh
                .electrical_customer_station_indoor_transformer_wall_buffer_ft;
    let tx_envelope_area_sf = tx_envelope_length_ft * tx_envelope_width_ft;
    let gear_envelope_length_ft = a.boh.electrical_customer_station_indoor_gear_envelope_length_ft;
    let gear_envelope_width_ft = a.boh.electrical_customer_station_indoor_gear_envelope_width_ft;
    let gear_envelope_area_sf = gear_envelope_length_ft * gear_envelope_width_ft;
    let service_aisle_ft = a.boh.electrical_customer_station_indoor_service_aisle_ft;
    let circulation_multiplier = 1.0 + a.boh.electrical_customer_station_indoor_circulation_ratio;
    let ancillary_fixed_per_floor_sf =
        a.boh.electrical_customer_station_indoor_ancillary_fixed_per_floor_sf;
    let packed_total_per_floor_sf = (tx_envelope_area_sf
        + gear_envelope_area_sf
        + service_aisle_ft * tx_envelope_length_ft.max(gear_envelope_length_ft))
        * circulation_multiplier
        + ancillary_fixed_per_floor_sf;
    let rect_total_per_floor_sf = ((tx_envelope_length_ft
        + gear_envelope_length_ft
        + service_aisle_ft)
        * tx_envelope_width_ft.max(gear_envelope_width_ft))
        * circulation_multiplier
        + ancillary_fixed_per_floor_sf;
    packed_total_per_floor_sf.min(rect_total_per_floor_sf)
        * a.boh.electrical_customer_station_indoor_transformer_vault_floors.max(1) as f64
}

fn selected_electrical_utility_infrastructure_exterior_transformer_option(
    required_kva: f64,
    a: &AssumptionPack,
) -> Option<(u32, &ElectricalTransformerSelectionOption)> {
    if required_kva <= EPS {
        return None;
    }
    let mut best: Option<(f64, u32, &ElectricalTransformerSelectionOption)> = None;
    for option in &a.boh.electrical_utility_infrastructure_exterior_transformer_options {
        let count = (required_kva / option.rating_kva.max(EPS))
            .ceil()
            .max(1.0) as u32;
        let total_cost = count as f64 * option.selection_cost_usd;
        if best.map_or(true, |(best_cost, _, _)| total_cost + EPS < best_cost) {
            best = Some((total_cost, count, option));
        }
    }
    best.map(|(_, count, option)| (count, option))
}

fn kva_area_guide_area_sf(kva: f64, guide: &[KvaSizedAreaOption]) -> f64 {
    guide
        .iter()
        .find(|point| kva <= point.max_kva + EPS)
        .or_else(|| guide.last())
        .map(|point| point.area_sf)
        .unwrap_or(0.0)
}

pub fn electrical_utility_infrastructure_exterior_required_kva(
    total_kva: f64,
    a: &AssumptionPack,
) -> f64 {
    if total_kva <= EPS {
        0.0
    } else {
        total_kva
            * 1.25
            * (1.0 + a
                .boh
                .electrical_utility_infrastructure_exterior_load_growth_ratio)
    }
}

pub fn electrical_utility_infrastructure_exterior_area_sf(
    total_kva: f64,
    a: &AssumptionPack,
) -> f64 {
    let required_kva = electrical_utility_infrastructure_exterior_required_kva(total_kva, a);
    if required_kva <= EPS {
        return 0.0;
    }
    // APT CA1 row 965: exterior utility infrastructure is needed whenever the
    // indoor customer-station reserve is off, or when the rated load is small
    // enough to stay below the indoor-station threshold.
    if a.boh.electrical_customer_station_indoor_enabled && required_kva > 249.0 + EPS {
        return 0.0;
    }
    let Some((count, option)) =
        selected_electrical_utility_infrastructure_exterior_transformer_option(required_kva, a)
    else {
        return 0.0;
    };
    let unit_area_sf = if option.rating_kva <= 300.0 + EPS {
        kva_area_guide_area_sf(
            option.rating_kva,
            &a.boh.electrical_utility_infrastructure_exterior_dry_area_guide,
        )
    } else {
        kva_area_guide_area_sf(
            option.rating_kva,
            &a.boh.electrical_utility_infrastructure_exterior_pad_area_guide,
        )
    };
    unit_area_sf * count as f64
        * (1.0 + a
            .boh
            .electrical_utility_infrastructure_exterior_layout_growth_ratio)
        + a.boh.electrical_utility_infrastructure_exterior_other_area_sf
}

pub fn electrical_elevator_load_kva(
    passenger_elevators: u32,
    freight_elevators: u32,
    a: &AssumptionPack,
) -> f64 {
    let elevator_count = passenger_elevators + freight_elevators;
    elevator_count as f64
        * a.boh.electrical_elevator_kva_per_car
        * electrical_elevator_demand_factor(elevator_count)
}

pub fn emergency_electrical_running_kva(
    passenger_elevators: u32,
    freight_elevators: u32,
    corridor_area_sf: f64,
    retail_area_sf: f64,
    shaft_stair_area_sf: f64,
    entry_lobby_area_sf: f64,
    indoor_parking_area_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    coarse_non_residential_electrical_load_kva(
        corridor_area_sf,
        retail_area_sf,
        shaft_stair_area_sf,
        entry_lobby_area_sf,
        indoor_parking_area_sf,
        a,
    ) + a.boh.electrical_generator_fire_alarm_kva
        + electrical_elevator_load_kva(passenger_elevators, freight_elevators, a)
}

pub fn generator_room_required_standby_kva(
    stories_above_grade: u32,
    passenger_elevators: u32,
    freight_elevators: u32,
    corridor_area_sf: f64,
    retail_area_sf: f64,
    shaft_stair_area_sf: f64,
    entry_lobby_area_sf: f64,
    indoor_parking_area_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    if stories_above_grade < a.boh.electrical_generator_room_enable_min_stories {
        return 0.0;
    }
    let emergency_running_kva = emergency_electrical_running_kva(
        passenger_elevators,
        freight_elevators,
        corridor_area_sf,
        retail_area_sf,
        shaft_stair_area_sf,
        entry_lobby_area_sf,
        indoor_parking_area_sf,
        a,
    );
    let mandatory_running_kva = a.boh.electrical_generator_smoke_control_kw
        / a.boh
            .electrical_generator_smoke_control_power_factor
            .max(EPS)
        * a.boh.electrical_generator_smoke_control_demand_factor;
    let optional_running_kva = a.boh.electrical_generator_domestic_booster_kw
        / a.boh
            .electrical_generator_domestic_booster_power_factor
            .max(EPS)
        * a.boh.electrical_generator_domestic_booster_demand_factor;
    let worst_case_inst_kva = [
        a.boh.electrical_generator_fire_pump_start_kva,
        a.boh.electrical_motor_constant_kva + a.boh.electrical_generator_emergency_start_kva,
        a.boh.electrical_motor_constant_kva
            + emergency_running_kva
            + a.boh.electrical_generator_mandatory_start_kva,
        a.boh.electrical_motor_constant_kva
            + emergency_running_kva
            + mandatory_running_kva
            + a.boh.electrical_generator_optional_start_kva,
    ]
    .into_iter()
    .fold(0.0, f64::max);
    worst_case_inst_kva / a.boh.electrical_generator_site_factor.max(EPS)
        * (1.0 + a.boh.electrical_generator_growth_ratio)
}

fn selected_generator_option(
    required_standby_kva: f64,
    a: &AssumptionPack,
) -> Option<(u32, &ElectricalGeneratorOption)> {
    if required_standby_kva <= EPS {
        return None;
    }
    let mut best: Option<(f64, u32, &ElectricalGeneratorOption)> = None;
    for option in &a.boh.electrical_generator_options {
        let count = (required_standby_kva / option.standby_rating_kva.max(EPS))
            .ceil()
            .max(1.0) as u32;
        let total_cost = count as f64 * option.installed_cost_usd;
        let replace = match best {
            None => true,
            Some((best_cost, best_count, best_option)) => {
                total_cost + EPS < best_cost
                    || ((total_cost - best_cost).abs() <= EPS
                        && (count < best_count
                            || (count == best_count
                                && option.standby_rating_kva < best_option.standby_rating_kva)))
            }
        };
        if replace {
            best = Some((total_cost, count, option));
        }
    }
    best.map(|(_, count, option)| (count, option))
}

pub fn generator_room_count(
    stories_above_grade: u32,
    passenger_elevators: u32,
    freight_elevators: u32,
    corridor_area_sf: f64,
    retail_area_sf: f64,
    shaft_stair_area_sf: f64,
    entry_lobby_area_sf: f64,
    indoor_parking_area_sf: f64,
    a: &AssumptionPack,
) -> u32 {
    let required_standby_kva = generator_room_required_standby_kva(
        stories_above_grade,
        passenger_elevators,
        freight_elevators,
        corridor_area_sf,
        retail_area_sf,
        shaft_stair_area_sf,
        entry_lobby_area_sf,
        indoor_parking_area_sf,
        a,
    );
    selected_generator_option(required_standby_kva, a)
        .map(|(count, _)| count)
        .unwrap_or(0)
}

pub fn generator_room_area_sf(
    stories_above_grade: u32,
    passenger_elevators: u32,
    freight_elevators: u32,
    corridor_area_sf: f64,
    retail_area_sf: f64,
    shaft_stair_area_sf: f64,
    entry_lobby_area_sf: f64,
    indoor_parking_area_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    let required_standby_kva = generator_room_required_standby_kva(
        stories_above_grade,
        passenger_elevators,
        freight_elevators,
        corridor_area_sf,
        retail_area_sf,
        shaft_stair_area_sf,
        entry_lobby_area_sf,
        indoor_parking_area_sf,
        a,
    );
    let Some((count, option)) = selected_generator_option(required_standby_kva, a) else {
        return 0.0;
    };
    count as f64
        * option.added_clearance_footprint_sf
        * (1.0 + a.boh.electrical_generator_growth_ratio)
}

fn ats_equipment_size_ft(load_kva: f64, a: &AssumptionPack) -> (f64, f64) {
    let amps = load_kva * 1000.0
        / (a.boh.electrical_service_input_voltage_v.max(EPS) * 3.0_f64.sqrt());
    let Some(option) = a
        .boh
        .electrical_ats_equipment_sizing
        .iter()
        .find(|option| amps <= option.max_amps)
        .or_else(|| a.boh.electrical_ats_equipment_sizing.first())
    else {
        return (0.0, 0.0);
    };
    (option.width_in / 12.0, option.depth_in / 12.0)
}

pub fn ats_room_area_sf(
    total_electrical_load_kva: f64,
    emergency_running_kva: f64,
    generator_count: u32,
    has_generator_room: bool,
    a: &AssumptionPack,
) -> f64 {
    if !has_generator_room {
        return 0.0;
    }
    let (main_w_ft, main_d_ft) = ats_equipment_size_ft(total_electrical_load_kva, a);
    let (emergency_w_ft, emergency_d_ft) = ats_equipment_size_ft(emergency_running_kva, a);
    let step_down_required =
        a.boh.electrical_generator_output_voltage_v > a.boh.electrical_service_input_voltage_v;
    let (step_down_w_ft, step_down_d_ft) =
        if total_electrical_load_kva <= a.boh.electrical_ats_step_down_small_max_kva {
            (
                a.boh.electrical_ats_step_down_small_width_in / 12.0,
                a.boh.electrical_ats_step_down_small_depth_in / 12.0,
            )
        } else {
            (
                a.boh.electrical_ats_step_down_large_width_in / 12.0,
                a.boh.electrical_ats_step_down_large_depth_in / 12.0,
            )
        };
    let qty = [
        1usize,
        1usize,
        a.boh.electrical_ats_ev_qty as usize,
        a.boh.electrical_ats_critical_qty as usize,
        step_down_required as usize,
        (generator_count > 2) as usize,
        1usize,
    ];
    let widths_ft = [
        main_w_ft,
        emergency_w_ft,
        0.0,
        0.0,
        step_down_w_ft,
        a.boh.electrical_ats_generator_distribution_width_in / 12.0,
        a.boh.electrical_ats_service_entrance_breaker_width_in / 12.0,
    ];
    let depths_ft = [
        main_d_ft,
        emergency_d_ft,
        0.0,
        0.0,
        step_down_d_ft,
        a.boh.electrical_ats_generator_distribution_depth_in / 12.0,
        a.boh.electrical_ats_service_entrance_breaker_depth_in / 12.0,
    ];
    let (room_width_ft, room_depth_ft) = f_roomsize(
        &qty,
        &widths_ft,
        &depths_ft,
        a.boh.electrical_ats_front_clear_ft,
        a.boh.electrical_ats_two_side_clear_ft,
        a.boh.electrical_ats_two_equipment_clear_ft,
    );
    room_width_ft * room_depth_ft * (1.0 + a.boh.electrical_ats_growth_ratio)
}

fn rule_of_thumb_room_dimensions_ft(
    load_kw: f64,
    guide: &[RuleOfThumbRoomGuidePoint],
) -> (f64, f64) {
    let Some(max_point) = guide.last() else {
        return (0.0, 0.0);
    };
    let ratio = max_point.depth_ft / max_point.width_ft.max(EPS);
    let area = max_point.width_ft * max_point.depth_ft;
    let fallback_depth_ft = ((load_kw / max_point.load_kw.max(EPS)) * area / ratio.max(EPS)).sqrt();
    let fallback_width_ft = fallback_depth_ft * ratio;
    if let Some(point) = guide.iter().find(|point| load_kw <= point.load_kw) {
        (point.width_ft, point.depth_ft)
    } else {
        (fallback_width_ft, fallback_depth_ft)
    }
}

pub fn emergency_lighting_inverter_room_area_sf(
    non_residential_electrical_load_kva: f64,
    has_generator_room: bool,
    units: u32,
    a: &AssumptionPack,
) -> f64 {
    if has_generator_room || units <= a.boh.electrical_elir_enable_min_units {
        return 0.0;
    }
    let lighting_load_kw =
        non_residential_electrical_load_kva * a.boh.electrical_elir_lighting_power_factor;
    let (room_width_ft, room_depth_ft) =
        rule_of_thumb_room_dimensions_ft(lighting_load_kw, &a.boh.electrical_elir_room_guide);
    room_width_ft * room_depth_ft
}

pub fn solar_battery_ups_room_area_sf(
    stories_above_grade: u32,
    units: u32,
    emergency_running_kva: f64,
    roof_area_sf: f64,
    available_roof_space_sf: Option<f64>,
    a: &AssumptionPack,
) -> f64 {
    if units < a.boh.electrical_ups_room_enable_min_units {
        return 0.0;
    }
    if stories_above_grade <= a.boh.electrical_ups_room_low_rise_max_stories {
        let Some(available_roof_space_sf) = available_roof_space_sf else {
            // APT CA1 only enables the low-rise branch when roof reserve has already
            // been checked. Keep that branch off until the roof-space signal is surfaced.
            return 0.0;
        };
        let roof_ratio = available_roof_space_sf / roof_area_sf.max(EPS);
        if roof_ratio + EPS < a.boh.electrical_ups_room_low_rise_min_available_roof_ratio {
            return 0.0;
        }
    }
    let required_backup_kwh = emergency_running_kva.max(0.0) * a.boh.electrical_ups_room_backup_time_hr;
    if required_backup_kwh <= EPS {
        return 0.0;
    }
    let design_storage_kwh = required_backup_kwh * a.boh.electrical_ups_room_capacity_factor;
    let battery_effective_kwh = a.boh.electrical_ups_room_battery_cabinet_kwh
        * a.boh.electrical_ups_room_battery_dod
        / a.boh.electrical_ups_room_battery_age_factor.max(EPS);
    let battery_qty = (design_storage_kwh / battery_effective_kwh.max(EPS))
        .ceil()
        .max(1.0) as usize;
    let power_qty = (required_backup_kwh / a.boh.electrical_ups_room_power_cabinet_kwh.max(EPS))
        .ceil()
        .max(1.0) as usize
        + a.boh.electrical_ups_room_power_cabinet_qty_offset as usize;
    let pcs_qty = (design_storage_kwh / a.boh.electrical_ups_room_pcs_kwh.max(EPS))
        .ceil()
        .max(1.0) as usize;
    let distribution_qty = (design_storage_kwh
        / a.boh.electrical_ups_room_distribution_cabinet_kwh.max(EPS))
        .ceil()
        .max(1.0) as usize;
    let qty = [battery_qty, power_qty, pcs_qty, distribution_qty];
    let widths_ft = [
        a.boh.electrical_ups_room_battery_width_in / 12.0,
        a.boh.electrical_ups_room_power_width_in / 12.0,
        a.boh.electrical_ups_room_pcs_width_in / 12.0,
        a.boh.electrical_ups_room_distribution_width_in / 12.0,
    ];
    let depths_ft = [
        a.boh.electrical_ups_room_battery_depth_in / 12.0,
        a.boh.electrical_ups_room_power_depth_in / 12.0,
        a.boh.electrical_ups_room_pcs_depth_in / 12.0,
        a.boh.electrical_ups_room_distribution_depth_in / 12.0,
    ];
    let (room_width_ft, room_depth_ft) = f_roomsize(
        &qty,
        &widths_ft,
        &depths_ft,
        a.boh.electrical_ups_room_front_clear_ft,
        a.boh.electrical_ups_room_two_side_clear_ft,
        a.boh.electrical_ups_room_between_equipment_clear_ft,
    );
    room_width_ft * room_depth_ft * (1.0 + a.boh.electrical_ups_room_hvac_egress_ratio)
}

fn lookup_elevator_machine_room_dimensions_ft(
    rated_load_lb: f64,
    guide: &[ElevatorMachineRoomGuidePoint],
) -> (f64, f64) {
    if guide.is_empty() {
        return (0.0, 0.0);
    }
    let required = rated_load_lb.max(0.0);
    let point = guide
        .iter()
        .find(|point| required <= point.max_rated_load_lb + EPS)
        .unwrap_or_else(|| guide.last().unwrap());
    (point.width_ft, point.depth_ft)
}

fn elevator_machine_room_base_footprint_ft(
    stories_above_grade: u32,
    passenger_elevators: u32,
    freight_elevators: u32,
    a: &AssumptionPack,
) -> Option<(f64, f64)> {
    if passenger_elevators == 0 && freight_elevators == 0 {
        return None;
    }
    let passenger_guide = if stories_above_grade <= a.vertical.low_rise_max_stories {
        &a.vertical.passenger_machine_room_hydraulic_guide
    } else {
        &a.vertical.passenger_machine_room_electric_guide
    };
    let (passenger_width_ft, passenger_depth_ft) = if passenger_elevators > 0 {
        lookup_elevator_machine_room_dimensions_ft(a.vertical.passenger_rated_load_lb, passenger_guide)
    } else {
        (0.0, 0.0)
    };
    let freight_width_ft = if freight_elevators > 0 {
        a.vertical.freight_machine_room_width_ft
    } else {
        0.0
    };
    let freight_depth_ft = if freight_elevators > 0 {
        a.vertical.freight_machine_room_depth_ft
    } else {
        0.0
    };
    let base_area_sf = passenger_width_ft * passenger_depth_ft + freight_width_ft * freight_depth_ft;
    if base_area_sf <= EPS {
        return None;
    }
    let min_width_ft = passenger_width_ft.max(freight_width_ft).max(base_area_sf.sqrt());
    Some((min_width_ft, base_area_sf / min_width_ft.max(EPS)))
}

pub fn elevator_machine_room_total_area_sf(
    stories_above_grade: u32,
    passenger_elevators: u32,
    freight_elevators: u32,
    a: &AssumptionPack,
) -> f64 {
    let Some((base_width_ft, base_depth_ft)) = elevator_machine_room_base_footprint_ft(
        stories_above_grade,
        passenger_elevators,
        freight_elevators,
        a,
    ) else {
        return 0.0;
    };
    let control_room_qty = if a.vertical.passenger_machine_room_on_roof {
        1.0
    } else {
        2.0
    };
    base_width_ft * base_depth_ft * control_room_qty
}

pub fn elevator_machine_room_interior_area_sf(
    stories_above_grade: u32,
    passenger_elevators: u32,
    freight_elevators: u32,
    a: &AssumptionPack,
) -> f64 {
    if a.vertical.passenger_machine_room_on_roof {
        0.0
    } else {
        elevator_machine_room_total_area_sf(
            stories_above_grade,
            passenger_elevators,
            freight_elevators,
            a,
        )
    }
}

pub fn elevator_machine_room_roof_area_sf(
    stories_above_grade: u32,
    passenger_elevators: u32,
    freight_elevators: u32,
    a: &AssumptionPack,
) -> f64 {
    if !a.vertical.passenger_machine_room_on_roof {
        0.0
    } else {
        elevator_machine_room_total_area_sf(
            stories_above_grade,
            passenger_elevators,
            freight_elevators,
            a,
        )
    }
}

pub fn elevator_machine_room_roof_footprint_ft(
    stories_above_grade: u32,
    passenger_elevators: u32,
    freight_elevators: u32,
    a: &AssumptionPack,
) -> Option<(f64, f64)> {
    if !a.vertical.passenger_machine_room_on_roof {
        return None;
    }
    elevator_machine_room_base_footprint_ft(
        stories_above_grade,
        passenger_elevators,
        freight_elevators,
        a,
    )
}

pub fn wheelchair_lift_count(total_stories: u32, units: u32, a: &AssumptionPack) -> u32 {
    // APT CA1 gates the lift off whenever the passenger elevator shaft is present.
    // In the workbook sample that shaft flag resolves to `stories > 1`, while the
    // explicit user toggle lives in APT SA. Keep this assumption-driven until that
    // user-facing switch is surfaced in the core input.
    if !a.vertical.wheelchair_lift_enabled || units == 0 || total_stories > 1 {
        return 0;
    }
    interpolated_count_from_units(
        units,
        a.vertical.wheelchair_lift_units_1,
        a.vertical.wheelchair_lift_qty_1,
        a.vertical.wheelchair_lift_units_2,
        a.vertical.wheelchair_lift_qty_2,
    )
}

pub fn wheelchair_lift_area_sf(total_stories: u32, units: u32, a: &AssumptionPack) -> f64 {
    let landing_area_sf = a
        .vertical
        .wheelchair_lift_landing_ewa_sf
        .max(a.vertical.wheelchair_lift_landing_code_min_sf);
    wheelchair_lift_count(total_stories, units, a) as f64
        * landing_area_sf
        * a.vertical.wheelchair_lift_stop_count.max(1) as f64
}

fn residential_bedroom_count(unit_counts: [u32; 4]) -> u32 {
    unit_counts[1] + 2 * unit_counts[2] + 3 * unit_counts[3]
}

fn workbook_outdoor_air_cfm_per_sf(
    outdoor_air_cfm_per_sf: f64,
    occupancy_density_per_1000_sf: f64,
    cfm_per_person: f64,
) -> f64 {
    outdoor_air_cfm_per_sf + occupancy_density_per_1000_sf * cfm_per_person / 1000.0
}

fn fan_room_ahu_has_detailed_amenity_rows(space_name: &str) -> bool {
    matches!(
        space_name.to_ascii_lowercase().as_str(),
        "art studio"
            | "bar / caf\u{e9} nook"
            | "business center / coworking"
            | "children’s play area"
            | "children's play area"
            | "commercial kitchen"
            | "community / multi-purpose room"
            | "concierge desk"
            | "conference room"
            | "dining room"
            | "dog washing room"
            | "fitness room"
            | "game room"
            | "golf simulator room"
            | "library / reading lounge"
            | "massage / treatment room"
            | "podcast room"
            | "recording room"
            | "sauna / steam room"
            | "shower / changing room"
            | "spa / wellness center"
            | "spin studio"
            | "theater / screening room"
            | "yoga / pilates room"
    )
}

fn fan_room_ahu_non_residential_cfm_per_sf_for_space(
    space_name: &str,
    a: &AssumptionPack,
) -> Option<f64> {
    let name = space_name.to_ascii_lowercase();
    match name.as_str() {
        // Workbook `APT CA1!F293:F332` resolves these rows by name. Keep the detailed
        // room coefficients here so the mainline can consume them directly from support
        // rows when those rows are surfaced.
        "entry wind lobby" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 30.0, 7.5)),
        "bicycle repair area" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 10.0, 5.0)),
        "bicycle room" => Some(0.06),
        "common laundry room" => Some(workbook_outdoor_air_cfm_per_sf(0.12, 5.0, 5.0)),
        "entry lobby" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 30.0, 7.5)),
        "mail room / mail area" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 20.0, 5.0)),
        "storage room - general / maintenance" => Some(0.06),
        "storage room - cold storage delivery room" => Some(0.12),
        "storage room - parcels" => Some(0.06),
        "leasing office" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 5.0, 5.0)),
        "manager's office" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 5.0, 5.0)),
        "parking control room" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 5.0, 5.0)),
        "cctv / it / security equipment rooms" => {
            Some(workbook_outdoor_air_cfm_per_sf(0.06, 5.0, 5.0))
        }
        "staff break room" => Some(workbook_outdoor_air_cfm_per_sf(0.12, 25.0, 5.0)),
        "amenity storage" | "amenity storage room" => Some(0.06),
        "art studio" => Some(workbook_outdoor_air_cfm_per_sf(0.18, 20.0, 10.0)),
        "bar / caf\u{e9} nook" => {
            Some(workbook_outdoor_air_cfm_per_sf(0.18, 100.0, 7.5))
        }
        "business center / coworking" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 25.0, 5.0)),
        "children’s play area" | "children's play area" => {
            Some(workbook_outdoor_air_cfm_per_sf(0.18, 25.0, 10.0))
        }
        "commercial kitchen" => Some(workbook_outdoor_air_cfm_per_sf(0.12, 20.0, 7.5)),
        "community / multi-purpose room" => {
            Some(workbook_outdoor_air_cfm_per_sf(0.06, 100.0, 7.5))
        }
        "concierge desk" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 5.0, 5.0)),
        "conference room" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 50.0, 5.0)),
        "dining room" => Some(workbook_outdoor_air_cfm_per_sf(0.18, 70.0, 7.5)),
        "dog washing room" => Some(workbook_outdoor_air_cfm_per_sf(0.18, 5.0, 5.0)),
        "fitness room" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 20.0, 20.0)),
        "game room" => Some(workbook_outdoor_air_cfm_per_sf(0.18, 70.0, 7.5)),
        "golf simulator room" => Some(workbook_outdoor_air_cfm_per_sf(0.18, 30.0, 20.0)),
        "library / reading lounge" => Some(workbook_outdoor_air_cfm_per_sf(0.12, 25.0, 5.0)),
        "massage / treatment room" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 10.0, 10.0)),
        "podcast room" | "recording room" => {
            Some(workbook_outdoor_air_cfm_per_sf(0.06, 5.0, 10.0))
        }
        "sauna / steam room" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 25.0, 5.0)),
        "shower / changing room" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 10.0, 10.0)),
        "spa / wellness center" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 20.0, 10.0)),
        "spin studio" | "yoga / pilates room" => {
            Some(workbook_outdoor_air_cfm_per_sf(0.06, 20.0, 20.0))
        }
        "theater / screening room" => Some(workbook_outdoor_air_cfm_per_sf(0.06, 150.0, 5.0)),
        _ => None,
    }
}

fn fan_room_ahu_indoor_amenity_cfm_per_sf_for_include_key(
    name: &str,
    a: &AssumptionPack,
) -> Option<f64> {
    let workbook_space_name = indoor_amenity_program_space_name_for_include_key(name)?;
    fan_room_ahu_non_residential_cfm_per_sf_for_space(workbook_space_name, a)
}

pub fn fan_room_ahu_indoor_amenity_supply_air_cfm(
    input: &NormalizedInput,
    indoor_amenity_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    if indoor_amenity_sf <= EPS {
        return 0.0;
    }
    let detailed_rows = indoor_amenity_detailed_program_areas(input, indoor_amenity_sf, a);
    if detailed_rows.is_empty() {
        return indoor_amenity_sf.max(0.0) * a.boh.mechanical_ahu_room_non_residential_air_cfm_per_sf;
    }
    let resolved_detail_area = detailed_rows.iter().map(|(_, area_sf)| *area_sf).sum::<f64>();
    let detailed_cfm = detailed_rows
        .iter()
        .filter_map(|(space_name, area_sf)| {
            fan_room_ahu_non_residential_cfm_per_sf_for_space(space_name, a)
                .map(|cfm_per_sf| cfm_per_sf * area_sf.max(0.0))
        })
        .sum::<f64>();
    let residual_generic_area = (indoor_amenity_sf - resolved_detail_area).max(0.0);
    detailed_cfm + residual_generic_area * a.boh.mechanical_ahu_room_non_residential_air_cfm_per_sf
}

pub fn fan_room_ahu_non_residential_supply_air_cfm(
    input: &NormalizedInput,
    indoor_amenity_sf: f64,
    corridor_area_sf: f64,
    retail_area_sf: f64,
    support_rows: &[SpaceDemandRow],
    a: &AssumptionPack,
) -> f64 {
    let has_detailed_amenity_rows = support_rows
        .iter()
        .any(|row| fan_room_ahu_has_detailed_amenity_rows(&row.space_name));
    let indoor_amenity_cfm = if has_detailed_amenity_rows {
        0.0
    } else {
        fan_room_ahu_indoor_amenity_supply_air_cfm(input, indoor_amenity_sf, a)
    };
    let support_rows_cfm = support_rows
        .iter()
        .filter_map(|row| {
            fan_room_ahu_non_residential_cfm_per_sf_for_space(&row.space_name, a)
            .map(|cfm_per_sf| row.area_sf.max(0.0) * cfm_per_sf)
        })
        .sum::<f64>();
    retail_area_sf.max(0.0) * workbook_outdoor_air_cfm_per_sf(0.12, 15.0, 7.5)
        + corridor_area_sf.max(0.0) * 0.06
        + indoor_amenity_cfm
        + support_rows_cfm
}

pub fn fan_room_ahu_total_supply_air_cfm(
    unit_counts: [u32; 4],
    net_residential_area_sf: f64,
    non_residential_supply_air_cfm: f64,
    indoor_parking_area_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    let bedroom_count = residential_bedroom_count(unit_counts);
    let residential_supply_air_cfm = a.boh.mechanical_ahu_room_residential_air_cfm_per_sf
        * net_residential_area_sf.max(0.0)
        + a.boh.mechanical_ahu_room_residential_air_cfm_per_bedroom
            * (bedroom_count as f64 + a.boh.mechanical_ahu_room_residential_bedroom_offset);
    let parking_air_cfm_per_sf = if a.boh.mechanical_ahu_room_use_cono_sensor_control {
        a.boh.mechanical_ahu_room_parking_air_cfm_per_sf_sensor_control
    } else {
        a.boh.mechanical_ahu_room_parking_air_cfm_per_sf_full_on
    };
    let parking_supply_air_cfm = indoor_parking_area_sf.max(0.0) * parking_air_cfm_per_sf;
    residential_supply_air_cfm + non_residential_supply_air_cfm + parking_supply_air_cfm
}

pub fn fan_room_ahu_room_area_sf(
    unit_counts: [u32; 4],
    net_residential_area_sf: f64,
    non_residential_supply_air_cfm: f64,
    indoor_parking_area_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    let total_supply_air_cfm = fan_room_ahu_total_supply_air_cfm(
        unit_counts,
        net_residential_area_sf,
        non_residential_supply_air_cfm,
        indoor_parking_area_sf,
        a,
    );
    if total_supply_air_cfm <= a.boh.mechanical_ahu_room_enable_min_supply_air_cfm {
        return 0.0;
    }
    let equipment_width_ft = a.boh.mechanical_ahu_room_width_base_ft
        + a.boh.mechanical_ahu_room_width_per_10k_cfm_ft * total_supply_air_cfm / 10_000.0;
    let equipment_depth_ft = a.boh.mechanical_ahu_room_equipment_depth_base_ft
        + a.boh.mechanical_ahu_room_equipment_depth_per_sqrt_1k_cfm_ft
            * (total_supply_air_cfm / 1000.0).sqrt();
    let room_width_ft =
        equipment_width_ft + (2.0 * a.boh.mechanical_ahu_room_side_clear_ft).max(equipment_depth_ft);
    let room_depth_ft = equipment_depth_ft
        + a.boh.mechanical_ahu_room_front_clear_ft
        + a.boh.mechanical_ahu_room_side_clear_ft;
    room_width_ft * room_depth_ft
}

pub fn mechanical_pad_outdoor_footprint_ft(
    units: u32,
    net_residential_area_sf: f64,
    roof_area_sf: f64,
    a: &AssumptionPack,
) -> Option<(f64, f64)> {
    if !a.boh.mechanical_pad_outdoor_enabled
        || units == 0
        || net_residential_area_sf <= EPS
        || roof_area_sf <= EPS
    {
        return None;
    }
    let use_split_module = units <= a.boh.mechanical_pad_outdoor_split_max_units
        && units as f64 * a.boh.mechanical_pad_outdoor_split_roof_sf_per_unit
            / roof_area_sf.max(EPS)
            <= a.boh.mechanical_pad_outdoor_split_max_roof_coverage_ratio + EPS;
    let (equipment_width_ft, equipment_depth_ft, equipment_qty) = if use_split_module {
        (
            a.boh.mechanical_pad_outdoor_split_width_ft,
            a.boh.mechanical_pad_outdoor_split_depth_ft,
            units,
        )
    } else {
        let qty = (net_residential_area_sf
            / a.boh
                .mechanical_pad_outdoor_residential_sf_per_ton
                .max(EPS)
            / a.boh.mechanical_pad_outdoor_vrf_tons.max(EPS))
        .ceil()
        .max(1.0) as u32;
        (
            a.boh.mechanical_pad_outdoor_vrf_width_ft,
            a.boh.mechanical_pad_outdoor_vrf_depth_ft,
            qty,
        )
    };
    let qty_per_column = ((equipment_qty as f64
        / a.boh.mechanical_pad_outdoor_layout_divisor.max(1.0))
    .sqrt()
    .round()
    .max(1.0)) as u32;
    let qty_per_row = (equipment_qty as f64 / qty_per_column.max(1) as f64)
        .ceil()
        .max(1.0) as u32;
    let width_ft = a.boh.mechanical_pad_outdoor_side_clear_ft * 2.0
        + qty_per_column.saturating_sub(1) as f64 * a.boh.mechanical_pad_outdoor_service_aisle_ft
        + qty_per_column as f64 * equipment_width_ft;
    let depth_ft = a.boh.mechanical_pad_outdoor_front_clear_ft * 2.0
        + qty_per_row.saturating_sub(1) as f64 * a.boh.mechanical_pad_outdoor_equipment_clear_ft
        + qty_per_row as f64 * equipment_depth_ft;
    Some((width_ft, depth_ft))
}

pub fn mechanical_pad_outdoor_area_sf(
    units: u32,
    net_residential_area_sf: f64,
    roof_area_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    mechanical_pad_outdoor_footprint_ft(units, net_residential_area_sf, roof_area_sf, a)
        .map(|(width_ft, depth_ft)| width_ft * depth_ft)
        .unwrap_or(0.0)
}

fn mechanical_ventilation_riser_residential_supply_air_cfm(
    unit_counts: [u32; 4],
    net_residential_area_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    let bedroom_count = residential_bedroom_count(unit_counts);
    a.boh.mechanical_ventilation_riser_residential_air_cfm_per_sf * net_residential_area_sf.max(0.0)
        + a.boh.mechanical_ventilation_riser_residential_air_cfm_per_bedroom
            * (bedroom_count as f64 + a.boh.mechanical_ventilation_riser_residential_bedroom_offset)
}

fn mechanical_ventilation_riser_exhaust_air_cfm(
    unit_counts: [u32; 4],
    a: &AssumptionPack,
) -> f64 {
    let total_units = unit_counts.iter().sum::<u32>() as f64;
    let total_bathrooms = unit_counts[0] as f64
        * a.boh.mechanical_ventilation_riser_bathrooms_per_studio as f64
        + unit_counts[1] as f64 * a.boh.mechanical_ventilation_riser_bathrooms_per_one_bedroom as f64
        + unit_counts[2] as f64 * a.boh.mechanical_ventilation_riser_bathrooms_per_two_bedroom as f64
        + unit_counts[3] as f64 * a.boh.mechanical_ventilation_riser_bathrooms_per_three_bedroom as f64;
    total_bathrooms * a.boh.mechanical_ventilation_riser_bathroom_exhaust_cfm
        + total_units * a.boh.mechanical_ventilation_riser_unit_kitchen_exhaust_cfm
}

fn mechanical_ventilation_riser_duct_shaft_area_sf(air_cfm: f64, a: &AssumptionPack) -> f64 {
    if air_cfm <= EPS {
        return 0.0;
    }
    let duct_area_sf = air_cfm / a.boh.mechanical_ventilation_riser_duct_velocity_fpm.max(EPS);
    let duct_diameter_in = (duct_area_sf * 4.0 / std::f64::consts::PI)
        .sqrt()
        .mul_add(12.0, 0.0)
        .ceil();
    let protected_width_in = duct_diameter_in
        + 2.0 * a.boh.mechanical_ventilation_riser_pipe_diameter_in
        + 2.0 * a.boh.mechanical_ventilation_riser_clearance_in;
    protected_width_in.powi(2) / 144.0
}

fn mechanical_ventilation_riser_ref_lines_area_sf(
    units: u32,
    typical_floor_area_sf: f64,
    net_residential_area_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    if units == 0 || typical_floor_area_sf <= EPS || net_residential_area_sf <= EPS {
        return 0.0;
    }
    // APT CA1 switches between 2-ton split heat-pump branches and 14-ton grouped
    // modules based on unit count and typical GFA density. Surface the exact branch
    // math here so the coarse shaft budget stays workbook-parity.
    let uses_small_split_modules = units <= a.boh.mechanical_ventilation_riser_small_units_max
        && units as f64 * a.boh.mechanical_ventilation_riser_small_density_sf_per_du
            / typical_floor_area_sf.max(EPS)
            <= a.boh.mechanical_ventilation_riser_small_density_max_ratio + EPS
        && (a.boh.mechanical_ventilation_riser_small_module_tons - 2.0).abs() <= EPS;
    if uses_small_split_modules {
        units as f64
            / a.boh
                .mechanical_ventilation_riser_small_module_qty_divisor
                .max(EPS)
            * a.boh.mechanical_ventilation_riser_small_module_area_sf
    } else {
        (net_residential_area_sf
            / a.boh
                .mechanical_ventilation_riser_large_module_coverage_sf
                .max(EPS))
        .ceil()
        .max(1.0)
            * a.boh.mechanical_ventilation_riser_large_module_area_sf
    }
}

pub fn mechanical_ventilation_riser_count(
    unit_counts: [u32; 4],
    units: u32,
    typical_floor_area_sf: f64,
    net_residential_area_sf: f64,
    fan_room_total_supply_air_cfm: f64,
    a: &AssumptionPack,
) -> u32 {
    if mechanical_ventilation_riser_area_sf(
        unit_counts,
        units,
        typical_floor_area_sf,
        net_residential_area_sf,
        fan_room_total_supply_air_cfm,
        a,
    ) > EPS
    {
        1
    } else {
        0
    }
}

pub fn mechanical_ventilation_riser_area_sf(
    unit_counts: [u32; 4],
    units: u32,
    typical_floor_area_sf: f64,
    net_residential_area_sf: f64,
    fan_room_total_supply_air_cfm: f64,
    a: &AssumptionPack,
) -> f64 {
    if fan_room_total_supply_air_cfm <= a.boh.mechanical_ventilation_riser_enable_min_supply_air_cfm
    {
        return 0.0;
    }
    let ref_lines_area_sf = mechanical_ventilation_riser_ref_lines_area_sf(
        units,
        typical_floor_area_sf,
        net_residential_area_sf,
        a,
    );
    if ref_lines_area_sf <= EPS {
        return 0.0;
    }
    let residential_supply_air_cfm =
        mechanical_ventilation_riser_residential_supply_air_cfm(unit_counts, net_residential_area_sf, a);
    let exhaust_air_cfm = mechanical_ventilation_riser_exhaust_air_cfm(unit_counts, a);
    let per_floor_area_sf = ref_lines_area_sf
        + mechanical_ventilation_riser_duct_shaft_area_sf(exhaust_air_cfm, a)
        + mechanical_ventilation_riser_duct_shaft_area_sf(residential_supply_air_cfm, a);
    per_floor_area_sf * ref_lines_area_sf
}

pub fn sub_electrical_room_count(
    stories_above_grade: u32,
    units: u32,
    gross_building_area_sf: f64,
    a: &AssumptionPack,
) -> u32 {
    let height_category = building_height_category(stories_above_grade, a);
    // APT CA1 also enables this room cluster above 500 kVA. We defer that gate
    // until a coarse electrical-load helper is surfaced into the core.
    let enabled = height_category == "high_rise"
        || units > a.boh.sub_electrical_enable_min_units
        || gross_building_area_sf > a.boh.sub_electrical_enable_min_gba_sf;
    if !enabled {
        return 0;
    }
    match height_category {
        "high_rise" => stories_above_grade.saturating_sub(1),
        "mid_rise" => ((stories_above_grade.saturating_sub(1) as f64) / 2.0).round() as u32,
        _ => 0,
    }
}

pub fn sub_electrical_room_single_area_sf(
    residential_story_count: u32,
    units: u32,
    a: &AssumptionPack,
) -> f64 {
    let units_per_floor = units as f64 / residential_story_count.max(1) as f64;
    let panelboards = (units_per_floor / a.boh.sub_electrical_units_per_panelboard.max(1.0))
        .ceil()
        .max(1.0);
    let option_1_depth_ft = a.boh.sub_electrical_panel_depth_ft
        + a.boh.sub_electrical_front_clear_ft
        + a.boh.sub_electrical_wiggle_ft;
    let option_1_length_ft =
        panelboards * a.boh.sub_electrical_panel_width_ft + a.boh.sub_electrical_side_clear_ft;
    let option_1_area_sf =
        option_1_depth_ft * option_1_length_ft * (1.0 + a.boh.sub_electrical_growth_ratio);
    let option_2_area_sf =
        a.boh.sub_electrical_option_2_w_ft * a.boh.sub_electrical_option_2_d_ft;
    let option_3_area_sf =
        a.boh.sub_electrical_option_3_w_ft * a.boh.sub_electrical_option_3_d_ft;
    if units_per_floor <= a.boh.sub_electrical_option_1_max_units_per_floor {
        option_1_area_sf
    } else if units_per_floor <= a.boh.sub_electrical_option_2_max_units_per_floor {
        option_2_area_sf
    } else {
        option_3_area_sf
    }
}

pub fn sub_electrical_rooms_area_sf(
    stories_above_grade: u32,
    residential_story_count: u32,
    units: u32,
    gross_building_area_sf: f64,
    a: &AssumptionPack,
) -> f64 {
    let qty = sub_electrical_room_count(stories_above_grade, units, gross_building_area_sf, a);
    if qty == 0 {
        return 0.0;
    }
    sub_electrical_room_single_area_sf(residential_story_count, units, a) * qty as f64
}

pub fn domestic_cold_water_fixture_units(
    counts: [u32; 4],
    public_wc_count: u32,
    public_urinal_count: u32,
    public_lav_count: u32,
    a: &AssumptionPack,
) -> f64 {
    let residential_wsfu = counts[0] as f64 * a.boh.domestic_water_booster_room_residential_wsfu_per_studio
        + counts[1] as f64 * a.boh.domestic_water_booster_room_residential_wsfu_per_one_bedroom
        + counts[2] as f64 * a.boh.domestic_water_booster_room_residential_wsfu_per_two_bedroom
        + counts[3] as f64 * a.boh.domestic_water_booster_room_residential_wsfu_per_three_bedroom;
    let public_wsfu = public_wc_count as f64 * a.boh.domestic_water_booster_room_public_wc_wsfu
        + public_urinal_count as f64 * a.boh.domestic_water_booster_room_public_urinal_wsfu
        + public_lav_count as f64 * a.boh.domestic_water_booster_room_public_lav_wsfu
        + a.boh.domestic_water_booster_room_public_kitchen_sink_qty as f64
            * a.boh.domestic_water_booster_room_public_kitchen_sink_wsfu
        + a.boh.domestic_water_booster_room_public_service_sink_qty as f64
            * a.boh.domestic_water_booster_room_public_service_sink_wsfu;
    (residential_wsfu + public_wsfu)
        * (1.0 + a.boh.domestic_water_booster_room_future_growth_ratio)
}

pub fn domestic_peak_flow_gpm(domestic_cold_water_fixture_units: f64, a: &AssumptionPack) -> f64 {
    let x = domestic_cold_water_fixture_units.max(0.0);
    a.boh.domestic_water_booster_room_peak_flow_cubic_a * x.powi(3)
        + a.boh.domestic_water_booster_room_peak_flow_cubic_b * x.powi(2)
        + a.boh.domestic_water_booster_room_peak_flow_cubic_c * x
        + a.boh.domestic_water_booster_room_peak_flow_cubic_d
        + a.boh.domestic_water_booster_room_continuous_demand_gpm
}

pub fn domestic_water_booster_pump_room_area_sf(
    total_story_count: u32,
    gross_building_area_sf: f64,
    domestic_peak_flow_gpm: f64,
    a: &AssumptionPack,
) -> f64 {
    if total_story_count < a.boh.domestic_water_booster_room_enable_min_stories
        || domestic_peak_flow_gpm <= EPS
    {
        return 0.0;
    }
    let typical_floor_area_sf = gross_building_area_sf / total_story_count.max(1) as f64;
    let equivalent_length_horizontal_ft = typical_floor_area_sf.max(0.0).sqrt() * 2.0;
    let building_height_ft =
        total_story_count as f64 * a.boh.domestic_water_booster_room_story_height_ft;
    let static_head_psi = building_height_ft / 2.31;
    let pipe_diameter_in = (domestic_peak_flow_gpm
        / a.boh
            .domestic_water_booster_room_pipe_capacity_constant
            .max(EPS)
        / a.boh
            .domestic_water_booster_room_pipe_velocity_fps
            .max(EPS))
    .sqrt();
    let equivalent_length_ft = (equivalent_length_horizontal_ft + building_height_ft)
        * (1.0 + a.boh.domestic_water_booster_room_fitting_friction_loss_factor);
    let piping_friction_loss_psi = (4.52
        * (domestic_peak_flow_gpm.powf(1.85) * equivalent_length_ft)
        / (a.boh
            .domestic_water_booster_room_hazen_williams_c
            .max(EPS)
            .powf(1.85)
            * pipe_diameter_in.max(EPS).powf(4.87)))
    .min(
        equivalent_length_ft / 100.0
            * a.boh
                .domestic_water_booster_room_friction_loss_max_psi_per_100ft,
    );
    let discharge_pressure_required_psi = static_head_psi
        + a.boh.domestic_water_booster_room_residual_pressure_psi
        + piping_friction_loss_psi
        + a.boh
            .domestic_water_booster_room_meter_backflow_prv_valves_psi
        + a.boh.domestic_water_booster_room_safety_margin_psi;
    let booster_boost_psi = discharge_pressure_required_psi
        - a.boh.domestic_water_booster_room_lowest_city_pressure_psi;
    let duty_pump_qty = if total_story_count
        >= a.boh
            .domestic_water_booster_room_high_story_three_pump_min_stories
    {
        3usize
    } else if domestic_peak_flow_gpm
        <= a.boh.domestic_water_booster_room_duty_pump_low_flow_max_gpm
    {
        1usize
    } else if domestic_peak_flow_gpm
        <= a.boh.domestic_water_booster_room_duty_pump_mid_flow_max_gpm
    {
        2usize
    } else {
        3usize
    };
    let minimum_flow_to_cover_gpm = a.boh.domestic_water_booster_room_run_fraction
        * domestic_peak_flow_gpm
        / duty_pump_qty.max(1) as f64;
    let minimum_run_time_min = if total_story_count
        <= a.boh.domestic_water_booster_room_low_story_max_for_longer_run
    {
        a.boh.domestic_water_booster_room_min_run_time_low_story_min
    } else {
        a.boh.domestic_water_booster_room_min_run_time_high_story_min
    };
    let drawdown_volume_gal = minimum_flow_to_cover_gpm * minimum_run_time_min;
    let expansion_tank_volume_gal = drawdown_volume_gal
        * (booster_boost_psi + 14.7)
        / a.boh
            .domestic_water_booster_room_expansion_tank_delta_p_psi
            .max(EPS);
    let expansion_tank_qty = (expansion_tank_volume_gal
        / a.boh
            .domestic_water_booster_room_expansion_tank_capacity_gal
            .max(EPS))
    .ceil()
    .max(1.0) as usize;
    let expansion_tank_diameter_in = (a.boh
        .domestic_water_booster_room_expansion_tank_diameter_curve_a
        * (expansion_tank_volume_gal / expansion_tank_qty.max(1) as f64).powf(
            a.boh
                .domestic_water_booster_room_expansion_tank_diameter_curve_b,
        )
        + a.boh
            .domestic_water_booster_room_expansion_tank_diameter_curve_c)
    .round();
    let qty = [
        duty_pump_qty + a.boh.domestic_water_booster_room_standby_pump_qty as usize,
        a.boh.domestic_water_booster_room_control_panel_qty as usize,
        expansion_tank_qty,
    ];
    let widths_ft = [
        a.boh.domestic_water_booster_room_pump_width_in / 12.0,
        a.boh.domestic_water_booster_room_control_panel_width_in / 12.0,
        expansion_tank_diameter_in / 12.0,
    ];
    let depths_ft = [
        a.boh.domestic_water_booster_room_pump_depth_in / 12.0,
        a.boh.domestic_water_booster_room_control_panel_depth_in / 12.0,
        expansion_tank_diameter_in / 12.0,
    ];
    let (room_width_ft, room_depth_ft) = f_roomsize(
        &qty,
        &widths_ft,
        &depths_ft,
        a.boh.domestic_water_booster_room_front_clear_ft,
        a.boh.domestic_water_booster_room_side_clear_ft,
        a.boh.domestic_water_booster_room_equipment_clear_ft,
    );
    room_width_ft * room_depth_ft
}

pub fn cistern_water_storage_tank_room_area_sf(
    domestic_peak_flow_gpm: f64,
    a: &AssumptionPack,
) -> f64 {
    if !a.boh.cistern_water_storage_room_enabled || domestic_peak_flow_gpm <= EPS {
        return 0.0;
    }
    domestic_peak_flow_gpm * a.boh.cistern_water_storage_room_a + a.boh.cistern_water_storage_room_b
}

fn lookup_diameter_sized_equipment(
    required_diameter_in: f64,
    guide: &[DiameterSizedEquipmentOption],
) -> (f64, f64) {
    if guide.is_empty() {
        return (0.0, 0.0);
    }
    let required = required_diameter_in.max(0.0);
    let option = guide
        .iter()
        .find(|option| required <= option.max_diameter_in + EPS)
        .unwrap_or_else(|| guide.last().unwrap());
    (option.width_in / 12.0, option.depth_in / 12.0)
}

fn backflow_preventer_domestic_qty_and_pipe_size_in(
    domestic_peak_flow_gpm: f64,
    a: &AssumptionPack,
) -> (usize, f64) {
    if domestic_peak_flow_gpm <= EPS {
        return (0, 0.0);
    }
    let q_max = a.boh.backflow_preventer_pipe_capacity_constant
        * a.boh.backflow_preventer_max_pipe_diameter_in.powi(2)
        * a.boh.backflow_preventer_pipe_velocity_fps;
    let qty = (domestic_peak_flow_gpm / q_max.max(EPS)).ceil().max(1.0) as usize;
    let pipe_size_in = (domestic_peak_flow_gpm
        / qty.max(1) as f64
        / a.boh.backflow_preventer_pipe_velocity_fps.max(EPS)
        / a.boh.backflow_preventer_pipe_capacity_constant.max(EPS))
    .sqrt();
    (qty, pipe_size_in)
}

pub fn backflow_preventer_room_area_sf(domestic_peak_flow_gpm: f64, a: &AssumptionPack) -> f64 {
    if !a.boh.backflow_preventer_room_enabled || domestic_peak_flow_gpm <= EPS {
        return 0.0;
    }
    let (domestic_qty, domestic_pipe_size_in) =
        backflow_preventer_domestic_qty_and_pipe_size_in(domestic_peak_flow_gpm, a);
    if domestic_qty == 0 {
        return 0.0;
    }
    let (domestic_width_ft, domestic_depth_ft) = lookup_diameter_sized_equipment(
        domestic_pipe_size_in,
        &a.boh.backflow_preventer_domestic_backflow_guide,
    );
    let (fire_width_ft, fire_depth_ft) = lookup_diameter_sized_equipment(
        domestic_pipe_size_in,
        &a.boh.backflow_preventer_fire_backflow_guide,
    );
    let qty = [
        domestic_qty,
        a.boh.backflow_preventer_fire_backflow_qty as usize,
        a.boh.backflow_preventer_irrigation_qty as usize,
    ];
    let widths_ft = [
        domestic_width_ft,
        fire_width_ft,
        a.boh.backflow_preventer_irrigation_width_in / 12.0,
    ];
    let depths_ft = [
        domestic_depth_ft,
        fire_depth_ft,
        a.boh.backflow_preventer_irrigation_depth_in / 12.0,
    ];
    let (room_width_ft, room_depth_ft) = f_roomsize(
        &qty,
        &widths_ft,
        &depths_ft,
        a.boh.backflow_preventer_front_clear_ft,
        a.boh.backflow_preventer_side_clear_ft,
        a.boh.backflow_preventer_equipment_clear_ft,
    );
    room_width_ft * room_depth_ft
}

pub fn central_water_heating_room_indoor_area_sf(a: &AssumptionPack) -> f64 {
    if !a.boh.central_water_heating_room_indoor_enabled {
        return 0.0;
    }
    a.boh.central_water_heating_room_sum_width_ft * a.boh.central_water_heating_room_a
        + a.boh.central_water_heating_room_sum_depth_ft * a.boh.central_water_heating_room_b
        + a.boh.central_water_heating_room_c
}

pub fn central_water_heating_pad_outdoor_area_sf(a: &AssumptionPack) -> f64 {
    if !a.boh.central_water_heating_pad_outdoor_enabled
        || a.boh.central_water_heating_room_indoor_enabled
    {
        return 0.0;
    }
    a.boh.central_water_heating_pad_outdoor_sum_width_ft * a.boh.central_water_heating_pad_outdoor_a
        + a.boh.central_water_heating_pad_outdoor_sum_depth_ft
            * a.boh.central_water_heating_pad_outdoor_b
        + a.boh.central_water_heating_pad_outdoor_c
}

pub fn graywater_system_room_area_sf(a: &AssumptionPack) -> f64 {
    if !a.boh.graywater_system_room_enabled {
        return 0.0;
    }
    a.boh.graywater_system_room_sum_width_ft * a.boh.graywater_system_room_a
        + a.boh.graywater_system_room_sum_depth_ft * a.boh.graywater_system_room_b
        + a.boh.graywater_system_room_c
}

pub fn water_filtration_area_sf(building_occupants: f64, a: &AssumptionPack) -> f64 {
    if !a.boh.water_filtration_enabled {
        return 0.0;
    }
    a.boh.water_filtration_a * building_occupants.max(0.0) + a.boh.water_filtration_b
}

pub fn grease_interceptor_room_area_sf(a: &AssumptionPack) -> f64 {
    if !a.boh.grease_interceptor_room_enabled {
        return 0.0;
    }
    a.boh.grease_interceptor_tank_size_gal * a.boh.grease_interceptor_a
        + a.boh.grease_interceptor_b
}

pub fn rainwater_harvesting_area_sf(a: &AssumptionPack) -> f64 {
    if !a.boh.rainwater_enabled {
        return 0.0;
    }
    a.boh.rainwater_sum_width_ft * a.boh.rainwater_a
        + a.boh.rainwater_sum_depth_ft * a.boh.rainwater_b
        + a.boh.rainwater_c
}

pub fn plumbing_riser_area_sf(units: u32, above_grade_stories: u32, a: &AssumptionPack) -> f64 {
    // APT CA1 carries an explicit "Has Shaft - Plumbing Risers?" gate ahead of the
    // regression row. In practice this resolves false for single-story projects, so
    // keep the coarse solver from seeding a vertical shaft when there is no vertical run.
    if !a.boh.plumbing_riser_enabled || above_grade_stories <= 1 {
        return 0.0;
    }
    (units as f64 * a.boh.plumbing_riser_units_a
        + above_grade_stories as f64 * a.boh.plumbing_riser_stories_b
        + a.boh.plumbing_riser_c)
        .max(0.0)
}

pub fn water_prv_closet_area_sf(above_grade_stories: u32, a: &AssumptionPack) -> f64 {
    if !a.boh.water_prv_closet_enabled
        || above_grade_stories < a.boh.water_prv_closet_enable_min_above_grade_stories
    {
        return 0.0;
    }
    above_grade_stories as f64 * a.boh.water_prv_closet_story_a + a.boh.water_prv_closet_b
}

pub fn fire_pump_room_area_sf(total_stories: u32, a: &AssumptionPack) -> f64 {
    if total_stories < a.boh.fire_pump_room_enable_min_total_stories {
        return 0.0;
    }
    let jockey_controller_zone_sf = a.boh.fire_pump_room_jockey_controller_width_ft
        * a.boh.fire_pump_room_jockey_controller_depth_ft;
    let fuel_tank_sf = if a.boh.fire_pump_room_diesel_fuel_tank_enabled {
        a.boh.fire_pump_room_design_fire_flow_gpm
            * a.boh.fire_pump_room_diesel_fuel_tank_plan_factor
    } else {
        0.0
    };
    let raw_pump_footprint_sf = (a.boh.fire_pump_room_pump_length_ft
        + a.boh.fire_pump_room_front_clear_ft
        + a.boh.fire_pump_room_side_back_clear_ft)
        * (a.boh.fire_pump_room_pump_width_ft + 2.0 * a.boh.fire_pump_room_side_back_clear_ft);
    let pump_skid_footprint_sf = raw_pump_footprint_sf
        .min(a.boh.fire_pump_room_max_sf)
        .max(a.boh.fire_pump_room_min_sf);
    pump_skid_footprint_sf + jockey_controller_zone_sf + fuel_tank_sf
}

pub fn fire_control_area_sf(total_stories: u32, building_gross_sf: f64, a: &AssumptionPack) -> f64 {
    if fire_pump_room_area_sf(total_stories, a) <= EPS {
        return 0.0;
    }
    (0.00015 * building_gross_sf).max(a.boh.fire_control_min_sf)
        + 20.0 * a.boh.fire_control_equipment_rack_count as f64
}

pub fn sprinkler_riser_closet_area_sf(total_stories: u32, a: &AssumptionPack) -> f64 {
    if total_stories < a.boh.sprinkler_riser_enable_max_stories_exclusive {
        a.boh.sprinkler_riser_default_sf
    } else {
        0.0
    }
}

pub fn gas_utility_meter_count(units: u32, a: &AssumptionPack) -> u32 {
    units.saturating_add(a.boh.gas_utility_master_meter_count)
}

pub fn gas_utility_meter_room_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    if !a.boh.gas_utility_room_enabled {
        return 0.0;
    }
    let meter_count = gas_utility_meter_count(units, a);
    let room_length = if meter_count <= 1 {
        a.boh.gas_utility_single_meter_length_ft
    } else {
        meter_count as f64 * a.boh.gas_utility_per_meter_length_ft
            + a.boh.gas_utility_length_offset_ft
    };
    a.boh.gas_utility_room_width_ft * room_length + a.boh.gas_utility_dcu_closet_sf
}

pub fn gas_meter_space_alcove_width_ft(units: u32, a: &AssumptionPack) -> f64 {
    let meter_count = gas_utility_meter_count(units, a);
    let bank_width_ft = match meter_count {
        0 | 1 => a.boh.gas_meter_space_alcove_single_bank_width_ft,
        2 => a.boh.gas_meter_space_alcove_two_meter_bank_width_ft,
        _ => {
            a.boh.gas_meter_space_alcove_two_meter_bank_width_ft
                + meter_count.saturating_sub(2) as f64
                    * a.boh.gas_meter_space_alcove_additional_meter_width_ft
        }
    };
    bank_width_ft + a.boh.gas_meter_space_alcove_side_clear_ft
}

pub fn gas_meter_space_alcove_area_sf(units: u32, a: &AssumptionPack) -> f64 {
    if !a.boh.gas_meter_space_alcove_enabled || a.boh.gas_utility_room_enabled {
        return 0.0;
    }
    let depth_ft =
        a.boh.gas_meter_space_alcove_bank_depth_ft + a.boh.gas_meter_space_alcove_front_clear_ft;
    depth_ft * gas_meter_space_alcove_width_ft(units, a)
}

/* ============================== cases / states ============================ */

#[derive(Debug, Clone)]
pub struct SpaceDemandRow {
    pub category: String,
    pub space_name: String,
    pub qty: f64,
    pub area_sf: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyOwnershipScopeKind {
    SiteOwned,
    FootprintOwned,
    FloorOwned,
    UnitOwned,
    RoomOwned,
    RoofOwned,
    StackOwned,
    ServiceOwned,
    ResidualOwned,
}

#[derive(Debug, Clone)]
pub struct SpaceSiblingConstraint {
    pub constraint_id: String,
    pub constraint_kind: String,
    pub sibling_space_id: String,
    pub required: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SpaceFlowRelation {
    pub relation_id: String,
    pub relation_kind: String,
    pub target_id: String,
    pub required: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SpaceServiceDependency {
    pub dependency_id: String,
    pub dependency_kind: String,
    pub target_id: String,
    pub required: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TopologyOwnershipRecord {
    pub owner_id: String,
    pub scope_kind: TopologyOwnershipScopeKind,
    pub parent_owner_id: Option<String>,
    pub level_index: Option<u32>,
    pub family_id: Option<String>,
    pub zone_owner_id: Option<String>,
    pub stack_id: Option<String>,
    pub owned_space_ids: Vec<String>,
    pub owned_node_ids: Vec<String>,
    pub child_owner_ids: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CirculationNodeKind {
    CorridorSegment,
    Junction,
    Decision,
    CoreAnchor,
    Door,
    DeadEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CirculationEdgeKind {
    SegmentConnection,
    JunctionBranch,
    DecisionBranch,
    DoorConnection,
    EntrySequence,
    PublicPrivateTransition,
    CoreTransfer,
}

#[derive(Debug, Clone)]
pub struct CirculationNode {
    pub node_id: String,
    pub level_index: u32,
    pub family_id: String,
    pub node_kind: CirculationNodeKind,
    pub polygon: Vec<Point2>,
    pub anchor: Point2,
    pub linked_topology_node_ids: Vec<String>,
    pub linked_space_ids: Vec<String>,
    pub branch_label: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CirculationEdge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub edge_kind: CirculationEdgeKind,
    pub directed: bool,
    pub length_ft: f64,
    pub linked_owner_id: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DoorToCorridorConnection {
    pub connection_id: String,
    pub unit_id: String,
    pub door_opening_id: String,
    pub door_node_id: String,
    pub corridor_segment_node_id: String,
    pub decision_node_id: Option<String>,
    pub distance_ft: f64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EntrySequenceGraph {
    pub sequence_id: String,
    pub unit_id: String,
    pub step_node_ids: Vec<String>,
    pub edge_ids: Vec<String>,
    pub public_node_ids: Vec<String>,
    pub private_node_ids: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PublicPrivateTransition {
    pub transition_id: String,
    pub unit_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub privacy_owner_id: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DeadEndOwnership {
    pub ownership_id: String,
    pub corridor_leaf_node_id: String,
    pub corridor_segment_node_id: String,
    pub owner_scope_id: String,
    pub owner_space_id: Option<String>,
    pub owner_topology_node_id: Option<String>,
    pub length_ft: f64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FloorCirculationTopology {
    pub level_index: u32,
    pub family_id: String,
    pub routing_mode: String,
    pub routing_classes: Vec<String>,
    pub corridor_segment_graph: Vec<CirculationNode>,
    pub junction_graph: Vec<CirculationNode>,
    pub decision_node_graph: Vec<CirculationNode>,
    pub edges: Vec<CirculationEdge>,
    pub door_to_corridor_connections: Vec<DoorToCorridorConnection>,
    pub entry_sequence_graphs: Vec<EntrySequenceGraph>,
    pub public_private_transition_graphs: Vec<PublicPrivateTransition>,
    pub dead_end_ownerships: Vec<DeadEndOwnership>,
    pub notes: Vec<String>,
}

impl FloorCirculationTopology {
    pub fn empty(level_index: u32, family_id: impl Into<String>) -> Self {
        Self {
            level_index,
            family_id: family_id.into(),
            routing_mode: "uninitialized".to_string(),
            routing_classes: Vec::new(),
            corridor_segment_graph: Vec::new(),
            junction_graph: Vec::new(),
            decision_node_graph: Vec::new(),
            edges: Vec::new(),
            door_to_corridor_connections: Vec::new(),
            entry_sequence_graphs: Vec::new(),
            public_private_transition_graphs: Vec::new(),
            dead_end_ownerships: Vec::new(),
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CirculationAnchorSeed {
    pub anchor_id: String,
    pub node_kind: CirculationNodeKind,
    pub anchor: Point2,
    pub polygon: Vec<Point2>,
    pub linked_topology_node_ids: Vec<String>,
    pub linked_space_ids: Vec<String>,
    pub unit_id: Option<String>,
    pub owner_scope_id: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SpacePlacement {
    pub space_id: String,
    pub category: String,
    pub space_name: String,
    pub qty: f64,
    pub rect: Rect2,
    pub polygon: Vec<Point2>,
    pub area_sf: f64,
    pub ownership_scope: TopologyOwnershipScopeKind,
    pub ownership_chain_ids: Vec<String>,
    pub parent_topology_node_id: Option<String>,
    pub child_topology_node_ids: Vec<String>,
    pub sibling_constraints: Vec<SpaceSiblingConstraint>,
    pub zone_owner_id: Option<String>,
    pub host_zone_id: Option<String>,
    pub concept_block_id: Option<String>,
    pub parent_space_id: Option<String>,
    pub stack_id: Option<String>,
    pub adjacency_ids: Vec<String>,
    pub ingress_relations: Vec<SpaceFlowRelation>,
    pub egress_relations: Vec<SpaceFlowRelation>,
    pub service_dependencies: Vec<SpaceServiceDependency>,
    pub daylight_owner_ids: Vec<String>,
    pub frontage_owner_ids: Vec<String>,
    pub privacy_owner_ids: Vec<String>,
    pub support_program: Option<SupportProgramKind>,
    pub support_profile: Option<SupportTopologyProfile>,
    pub residual_source_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalTopologyNodeKind {
    Zone,
    Room,
    CorridorSegment,
    Core,
    Shaft,
    Entry,
    OutdoorCell,
    SupportSpace,
    SiteRoute,
    RoofZone,
    VerticalSystem,
    ResidualCell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalTopologyEdgeKind {
    Adjacency,
    Access,
    ServiceReach,
    FrontageReach,
    VerticalContinuity,
    DaylightOwnership,
    PrivacyBoundary,
    Constraint,
    SeparationBuffer,
    DrainageDependency,
    MaintenanceAccess,
    PedestrianFlow,
    ServiceFlow,
    ParkingWalk,
    TransferLink,
}

#[derive(Debug, Clone)]
pub struct CanonicalTopologyNode {
    pub node_id: String,
    pub level_index: Option<u32>,
    pub family_id: Option<String>,
    pub node_kind: CanonicalTopologyNodeKind,
    pub semantic_label: String,
    pub parent_node_id: Option<String>,
    pub polygon: Vec<Point2>,
    pub guide_line: Option<Line2>,
    pub area_sf: f64,
    pub source_refs: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CanonicalTopologyEdge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub edge_kind: CanonicalTopologyEdgeKind,
    pub directed: bool,
    pub required: bool,
    pub span_ft: f64,
    pub source_refs: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TopologyGeometryOwnership {
    pub binding_id: String,
    pub owner_node_id: String,
    pub geometry_kind: String,
    pub geometry_ref: String,
    pub level_index: Option<u32>,
    pub source_ref: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CanonicalTopologyGraph {
    pub graph_id: String,
    pub scope_kind: String,
    pub level_index: Option<u32>,
    pub family_id: Option<String>,
    pub topology_stage: String,
    pub nodes: Vec<CanonicalTopologyNode>,
    pub edges: Vec<CanonicalTopologyEdge>,
    pub ownership_hierarchy: Vec<TopologyOwnershipRecord>,
    pub geometry_ownership: Vec<TopologyGeometryOwnership>,
    pub validation_issues: Vec<ValidationIssue>,
    pub identity_records: Vec<TopologyIdentityRecord>,
    pub provenance_records: Vec<TopologyProvenanceRecord>,
    pub contract_graph: TopologyConstraintGraph,
    pub repair_journal: TopologyRepairJournal,
    pub notes: Vec<String>,
}

impl CanonicalTopologyGraph {
    pub fn empty(graph_id: impl Into<String>, scope_kind: &str, topology_stage: &str) -> Self {
        let graph_id = graph_id.into();
        Self {
            graph_id: graph_id.clone(),
            scope_kind: scope_kind.to_string(),
            level_index: None,
            family_id: None,
            topology_stage: topology_stage.to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
            ownership_hierarchy: Vec::new(),
            geometry_ownership: Vec::new(),
            validation_issues: Vec::new(),
            identity_records: Vec::new(),
            provenance_records: Vec::new(),
            contract_graph: TopologyConstraintGraph::empty(
                format!("{}_contract", graph_id),
                scope_kind.to_string(),
            ),
            repair_journal: TopologyRepairJournal::empty(format!("{}_repair", graph_id)),
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConceptBlockRole {
    PrimaryMass,
    Wing,
    Branch,
    Bridge,
    CourtyardEdge,
    ServiceSpine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockBindingTransitionKind {
    Introduced,
    Persistent,
    Tapered,
    Expanded,
    Suppressed,
}

#[derive(Debug, Clone)]
pub struct ConceptBlock {
    pub block_id: String,
    pub source_shape: BuildingShape,
    pub ordinal_index: usize,
    pub primary_role: ConceptBlockRole,
    pub service_capable: bool,
    pub courtyard_edge: bool,
    pub branch_like: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConceptFamilyBlockBinding {
    pub binding_id: String,
    pub family_id: String,
    pub block_id: String,
    pub level_indices: Vec<u32>,
    pub rect_seed: Rect2,
    pub polygon: Vec<Point2>,
    pub area_sf: f64,
    pub active: bool,
    pub transition_kind: BlockBindingTransitionKind,
    pub inherited_from_family_id: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FloorFamily {
    pub family_id: String,
    pub level_indices: Vec<u32>,
    pub is_typical: bool,
    pub area_budget_sf: f64,
    pub uses_upper_footprint: bool,
    pub family_role: String,
    pub bound_block_ids: Vec<String>,
    pub binding_notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VerticalGroup {
    pub name: String,
    pub anchor: Point2,
    pub served_floor_min: u32,
    pub served_floor_max: u32,
    pub width_ft: f64,
    pub depth_ft: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SitePlanProgramKind {
    ArrivalForecourt,
    LoadingZone,
    ServiceYard,
    FireAccessBand,
    PublicWalk,
    AccessibleWalk,
    ParkingSurface,
    DriveAisle,
    ParkingWalk,
    LandscapeZone,
    OpenSpaceZone,
    PrivacyBuffer,
    ResidualDevelopable,
    BuildingFootprint,
    PodiumEnvelope,
    BelowGradeParkingEnvelope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SitePlanFrontageRole {
    PublicEntry,
    ServiceLoading,
    FireAccess,
    ParkingAccess,
    PrivacySensitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SitePlanAnchorKind {
    Arrival,
    BuildingEntry,
    Loading,
    Service,
    FireAccess,
    ParkingEntry,
    PublicWalk,
    AccessibleWalk,
    Landscape,
    Privacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SitePlanSegmentKind {
    PublicWalk,
    AccessibleWalk,
    ServiceFlow,
    FireFlow,
    ParkingWalk,
    DriveAisle,
    SeparationBuffer,
    ConflictCrossing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SitePlanManeuverClass {
    ParkingIngressEgress,
    LoadingServiceTurn,
    FireAccessTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SitePlanCheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone)]
pub struct SiteBuildableEnvelope {
    pub envelope_id: String,
    pub parcel_polygon: Vec<Point2>,
    pub buildable_polygon: Vec<Point2>,
    pub frontage_edge_indices: Vec<usize>,
    pub no_build_edge_indices: Vec<usize>,
    pub fallback_mode: String,
    pub issue_codes: Vec<String>,
    pub confidence: f64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SiteFrontageCandidate {
    pub frontage_id: String,
    pub edge_index: usize,
    pub start: Point2,
    pub end: Point2,
    pub length_ft: f64,
    pub public_score: f64,
    pub service_score: f64,
    pub fire_score: f64,
    pub parking_score: f64,
    pub privacy_score: f64,
    pub active_roles: Vec<SitePlanFrontageRole>,
    pub accepted: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SiteReservation {
    pub reservation_id: String,
    pub program_kind: SitePlanProgramKind,
    pub priority_rank: usize,
    pub target_area_sf: f64,
    pub reserved_area_sf: f64,
    pub perimeter_claim_ft: f64,
    pub linked_frontage_ids: Vec<String>,
    pub shortfall_area_sf: f64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SiteAnchorPoint {
    pub anchor_id: String,
    pub anchor_kind: SitePlanAnchorKind,
    pub point: Point2,
    pub linked_frontage_id: Option<String>,
    pub linked_reservation_id: Option<String>,
    pub required: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SiteSegment {
    pub segment_id: String,
    pub segment_kind: SitePlanSegmentKind,
    pub from_anchor_id: String,
    pub to_anchor_id: String,
    pub geometry: Vec<Point2>,
    pub width_ft: f64,
    pub required: bool,
    pub linked_zone_ids: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SiteZone {
    pub zone_id: String,
    pub zone_kind: SitePlanProgramKind,
    pub polygon: Vec<Point2>,
    pub area_sf: f64,
    pub linked_frontage_ids: Vec<String>,
    pub linked_anchor_ids: Vec<String>,
    pub linked_segment_ids: Vec<String>,
    pub residual: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SiteParkingAllocation {
    pub allocation_id: String,
    pub parking_mode: ParkingMode,
    pub reserved_stalls: u32,
    pub accessible_stalls: u32,
    pub reserved_area_sf: f64,
    pub active: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SiteParkingLotCell {
    pub cell_id: String,
    pub polygon: Vec<Point2>,
    pub stall_count_estimate: u32,
    pub accessible_stalls: u32,
    pub drive_aisle_segment_ids: Vec<String>,
    pub parking_walk_segment_ids: Vec<String>,
    pub fragmented: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SiteParkingTopology {
    pub topology_id: String,
    pub allocations: Vec<SiteParkingAllocation>,
    pub lot_cells: Vec<SiteParkingLotCell>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SiteManeuverCheck {
    pub check_id: String,
    pub maneuver_class: SitePlanManeuverClass,
    pub status: SitePlanCheckStatus,
    pub anchor_ids: Vec<String>,
    pub clearance_ft: f64,
    pub blocking_zone_ids: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SiteClearanceCheck {
    pub check_id: String,
    pub segment_id: Option<String>,
    pub zone_id: Option<String>,
    pub status: SitePlanCheckStatus,
    pub required_clear_width_ft: f64,
    pub provided_clear_width_ft: f64,
    pub blocking_refs: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SiteSimpleVolume {
    pub volume_id: String,
    pub zone_kind: SitePlanProgramKind,
    pub footprint_polygon: Vec<Point2>,
    pub base_level_index: Option<u32>,
    pub top_level_index: Option<u32>,
    pub height_ft: f64,
    pub below_grade: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SitePlanScoreBreakdown {
    pub far_priority_score: f64,
    pub dwelling_priority_score: f64,
    pub site_feasibility_penalty: f64,
    pub access_penalty: f64,
    pub parking_penalty: f64,
    pub privacy_penalty: f64,
    pub clearance_penalty: f64,
    pub total_score: f64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MassingSitePlanBundle {
    pub bundle_id: String,
    pub california_site_mode: bool,
    pub overlay_binding_mode: SiteOverlayBindingMode,
    pub overlay_reference: Option<String>,
    pub buildable_envelope: SiteBuildableEnvelope,
    pub frontage_candidates: Vec<SiteFrontageCandidate>,
    pub reservations: Vec<SiteReservation>,
    pub anchor_points: Vec<SiteAnchorPoint>,
    pub segments: Vec<SiteSegment>,
    pub site_zones: Vec<SiteZone>,
    pub parking_topology: SiteParkingTopology,
    pub maneuver_checks: Vec<SiteManeuverCheck>,
    pub clearance_checks: Vec<SiteClearanceCheck>,
    pub outdoor_topology_graph: OutdoorSiteTopologyGraph,
    pub concept_volumes: Vec<SiteSimpleVolume>,
    pub score_breakdown: SitePlanScoreBreakdown,
    pub diagnostics: Vec<ValidationIssue>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MassingState {
    pub building_shape: BuildingShape,
    pub site_area_sf: f64,
    pub site_perimeter_ft: f64,
    pub gfa_goal_sf: f64,
    pub preliminary_area_budget: PreliminaryAreaBudget,
    pub footprint_seed_sf: f64,
    pub podium_footprint_sf: f64,
    pub upper_footprint_sf: f64,
    pub story_count: u32,
    pub construction_case: ConstructionCase,
    pub shape_case: ShapeCase,
    pub shape_diagnostics: ShapeRealizationDiagnostics,
    pub concept_blocks: Vec<ConceptBlock>,
    pub family_block_bindings: Vec<ConceptFamilyBlockBinding>,
    pub site_plan_bundle: MassingSitePlanBundle,
}

#[derive(Debug, Clone)]
pub struct ProgramState {
    pub unit_areas_sf: [f64; 4],
    pub unit_mix_resolved: UnitMix,
    pub unit_counts: [u32; 4],
    pub dwelling_units_total: u32,
    pub avg_unit_area_sf: f64,
    pub coarse_total_electrical_load_kva: f64,
    pub preliminary_area_budget: PreliminaryAreaBudget,
    pub net_residential_area_sf: f64,
    pub retail_area_sf: f64,
    pub indoor_amenity_target_sf: f64,
    pub outdoor_amenity_target_sf: f64,
    pub support_rows: Vec<SpaceDemandRow>,
    pub support_topology: SupportTopologyState,
}

#[derive(Debug, Clone)]
pub struct VerticalState {
    pub corridor_width_ft: f64,
    pub passenger_elevators: u32,
    pub freight_elevators: u32,
    pub stairs: u32,
    pub groups: Vec<VerticalGroup>,
    pub topology_state: VerticalTopologyState,
}

#[derive(Debug, Clone)]
pub struct ZoningState {
    pub families: Vec<FloorFamily>,
}

#[derive(Debug, Clone)]
pub struct CandidateCase {
    pub candidate_id: String,
    pub building_shape: BuildingShape,
    pub story_count: u32,
    pub podium_levels: u32,
    pub corridor_type: CorridorType,
    pub core_strategy: CoreStrategy,
    pub shape_case: ShapeCase,
    pub construction_case: ConstructionCase,
    pub footprint_seed_sf: f64,
    pub podium_footprint_sf: f64,
    pub upper_footprint_sf: f64,
    pub shape_diagnostics: ShapeRealizationDiagnostics,
    pub site_plan_bundle: MassingSitePlanBundle,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloorZoneKind {
    Dwelling,
    Circulation,
    CoreService,
    Parking,
    Support,
    OutdoorCapable,
    RoofAmenity,
    Site,
}

#[derive(Debug, Clone)]
pub struct FloorZone {
    pub zone_id: String,
    pub level_index: u32,
    pub family_id: String,
    pub kind: FloorZoneKind,
    pub polygon: Vec<Point2>,
    pub rect: Rect2,
    pub area_sf: f64,
    pub concept_block_id: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FloorOutput {
    pub level_index: u32,
    pub family_id: String,
    pub inherited_from_level_index: Option<u32>,
    pub variant_kind: String,
    pub override_notes: Vec<String>,
    pub footprint_polygon: Vec<Point2>,
    pub footprint_components: Vec<Vec<Point2>>,
    pub footprint_voids: Vec<Vec<Point2>>,
    pub concept_block_ids: Vec<String>,
    pub zones: Vec<FloorZone>,
    pub guide_lines: Vec<Line2>,
    pub spaces: Vec<SpacePlacement>,
    pub room_spaces: Vec<SpacePlacement>,
    pub canonical_topology_graph: CanonicalTopologyGraph,
    pub circulation_topology: FloorCirculationTopology,
}

#[derive(Debug, Clone)]
pub struct ShapeRealizationDiagnostics {
    pub seed_area_sf: f64,
    pub realized_area_sf: f64,
    pub realized_frontage_ft: f64,
    pub clipping_loss_ratio: f64,
    pub parcel_fit_ratio: f64,
    pub fragment_count: usize,
    pub warnings: Vec<String>,
}

impl Default for ShapeRealizationDiagnostics {
    fn default() -> Self {
        Self {
            seed_area_sf: 0.0,
            realized_area_sf: 0.0,
            realized_frontage_ft: 0.0,
            clipping_loss_ratio: 0.0,
            parcel_fit_ratio: 0.0,
            fragment_count: 0,
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateFeasibilityStatus {
    Accepted,
    ManualReview,
    Rejected,
}

#[derive(Debug, Clone)]
pub struct CandidateFeasibility {
    pub status: CandidateFeasibilityStatus,
    pub hard_failure_count: usize,
    pub soft_issue_count: usize,
    pub penalty_total: f64,
    pub rejection_reasons: Vec<String>,
}

impl Default for CandidateFeasibility {
    fn default() -> Self {
        Self {
            status: CandidateFeasibilityStatus::Accepted,
            hard_failure_count: 0,
            soft_issue_count: 0,
            penalty_total: 0.0,
            rejection_reasons: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CandidateScoreBreakdown {
    pub concept_pre_score: f64,
    pub solved_far_score: f64,
    pub solved_dwelling_units_score: f64,
    pub solved_yield_score: f64,
    pub solved_amenity_score: f64,
    pub solved_repeatability_score: f64,
    pub access_penalty: f64,
    pub service_penalty: f64,
    pub parking_penalty: f64,
    pub residual_penalty: f64,
    pub validation_penalty: f64,
    pub drift_penalty: f64,
    pub total_score: f64,
}

impl Default for CandidateScoreBreakdown {
    fn default() -> Self {
        Self {
            concept_pre_score: 0.0,
            solved_far_score: 0.0,
            solved_dwelling_units_score: 0.0,
            solved_yield_score: 0.0,
            solved_amenity_score: 0.0,
            solved_repeatability_score: 0.0,
            access_penalty: 0.0,
            service_penalty: 0.0,
            parking_penalty: 0.0,
            residual_penalty: 0.0,
            validation_penalty: 0.0,
            drift_penalty: 0.0,
            total_score: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CandidateDiagnostics {
    pub target_far: f64,
    pub realized_far: f64,
    pub target_dwelling_units: u32,
    pub realized_dwelling_units: u32,
    pub realized_dwelling_mix: [u32; 4],
    pub realized_avg_unit_area_sf: f64,
    pub support_space_count: usize,
    pub unresolved_entry_count: usize,
    pub unresolved_service_count: usize,
    pub daylight_miss_count: usize,
    pub residual_space_count: usize,
    pub residual_area_sf: f64,
    pub parking_required_stalls: usize,
    pub parking_provided_stalls: usize,
    pub accessible_stalls: usize,
    pub parking_shortfall: isize,
    pub parking_efficiency: f64,
    pub validation_warning_count: usize,
    pub validation_error_count: usize,
    pub shape_realization: ShapeRealizationDiagnostics,
}

impl Default for CandidateDiagnostics {
    fn default() -> Self {
        Self {
            target_far: 0.0,
            realized_far: 0.0,
            target_dwelling_units: 0,
            realized_dwelling_units: 0,
            realized_dwelling_mix: [0; 4],
            realized_avg_unit_area_sf: 0.0,
            support_space_count: 0,
            unresolved_entry_count: 0,
            unresolved_service_count: 0,
            daylight_miss_count: 0,
            residual_space_count: 0,
            residual_area_sf: 0.0,
            parking_required_stalls: 0,
            parking_provided_stalls: 0,
            accessible_stalls: 0,
            parking_shortfall: 0,
            parking_efficiency: 0.0,
            validation_warning_count: 0,
            validation_error_count: 0,
            shape_realization: ShapeRealizationDiagnostics::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CandidateSolution {
    pub candidate_id: String,
    pub building_shape: BuildingShape,
    pub story_count: u32,
    pub corridor_type: CorridorType,
    pub core_strategy: CoreStrategy,
    pub concept_score: f64,
    pub score_total: f64,
    pub score_far: f64,
    pub score_dwelling_units: f64,
    pub score_yield: f64,
    pub score_amenity: f64,
    pub score_repeatability: f64,
    pub score_breakdown: CandidateScoreBreakdown,
    pub diagnostics: CandidateDiagnostics,
    pub feasibility: CandidateFeasibility,
    pub footprint_polygon: Vec<Point2>,
    pub floors: Vec<FloorOutput>,
    pub canonical_topology_graph: CanonicalTopologyGraph,
    pub repeatability_score: f64,
    pub footprint_area_sf: f64,
    pub gross_floor_area_sf: f64,
    pub validation_issues: Vec<ValidationIssue>,
    pub drift_metrics: Vec<DetailDriftMetric>,
    pub site_plan_bundle: Option<MassingSitePlanBundle>,
    pub site_spaces: Vec<SpacePlacement>,
    pub roof_spaces: Vec<SpacePlacement>,
    pub outdoor_site_topology_graph: Option<OutdoorSiteTopologyGraph>,
    pub roof_topology_graphs: Vec<RoofTopologyGraph>,
    pub notes: Vec<String>,
}

/* final output */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutEngineLayer {
    CoreLayout,
    RepoIntegration,
    OverlayAware,
    SpatialGeometry,
    SpatialTopology,
    ShapeStrategy,
    ConstructiveTopology,
    LayeredTopology,
    StructuralTopology,
    OpeningAware,
    ProgressiveTopology,
    AllocationReadiness,
    RoomSynthesis,
    ProgressiveSolve,
}

impl LayoutEngineLayer {
    pub const fn as_str(self) -> &'static str {
        match self {
            LayoutEngineLayer::CoreLayout => "core_layout",
            LayoutEngineLayer::RepoIntegration => "repo_integration",
            LayoutEngineLayer::OverlayAware => "overlay_aware",
            LayoutEngineLayer::SpatialGeometry => "spatial_geometry",
            LayoutEngineLayer::SpatialTopology => "spatial_topology",
            LayoutEngineLayer::ShapeStrategy => "shape_strategy",
            LayoutEngineLayer::ConstructiveTopology => "constructive_topology",
            LayoutEngineLayer::LayeredTopology => "layered_topology",
            LayoutEngineLayer::StructuralTopology => "structural_topology",
            LayoutEngineLayer::OpeningAware => "opening_aware",
            LayoutEngineLayer::ProgressiveTopology => "progressive_topology",
            LayoutEngineLayer::AllocationReadiness => "allocation_readiness",
            LayoutEngineLayer::RoomSynthesis => "room_synthesis",
            LayoutEngineLayer::ProgressiveSolve => "progressive_solve",
        }
    }
}

impl std::fmt::Display for LayoutEngineLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct LayoutOutput {
    pub engine_layer: LayoutEngineLayer,
    pub solution_status: SolutionStatus,
    pub validation_issues: Vec<ValidationIssue>,
    pub resolved_code_profile: JurisdictionCodePack,
    pub normalized_input: NormalizedInput,
    pub assumed_parameters: Vec<ResolvedVariable>,
    pub computed_parameters: Vec<ResolvedVariable>,
    pub resolved_variables: Vec<ResolvedVariable>,
    pub override_trace: Vec<OverrideTraceRecord>,
    pub massing: MassingState,
    pub program: ProgramState,
    pub verticals: VerticalState,
    pub zoning: ZoningState,
    pub candidate_solutions: Vec<CandidateSolution>,
    pub selected_candidate: CandidateSolution,
    pub canonical_topology_graph: CanonicalTopologyGraph,
    pub per_floor_canonical_topology_graphs: Vec<CanonicalTopologyGraph>,
    pub per_floor_circulation_topologies: Vec<FloorCirculationTopology>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitRoomSemanticKind {
    Foyer,
    Living,
    Dining,
    Kitchen,
    Bath,
    Bedroom,
    Closet,
    LaundryNiche,
    MechanicalCloset,
    PrivateHall,
    CompactFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitRoomPartition {
    Public,
    Private,
    Service,
    Circulation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitTopologyEdgeKind {
    Access,
    PrivacyBoundary,
    WetAdjacency,
    DaylightClaim,
    StructureCompatibility,
    EntrySequence,
    PublicPrivateTransition,
    LivingChain,
    VerticalDependency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyEntityKind {
    Graph,
    Node,
    Edge,
    Owner,
    GeometryBinding,
    Constraint,
    Violation,
    RepairTransaction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyProvenanceKind {
    LayoutPass,
    SpaceSeed,
    RoomSeed,
    SupportProgram,
    OutdoorZone,
    RoofZone,
    VerticalSystem,
    ResidualExtraction,
    RepresentativeProjection,
    DirtyRegionRepair,
    ManualFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyConstraintKind {
    MustTouch,
    MustNotTouch,
    AccessRequired,
    FrontageRequired,
    DaylightRequired,
    StackedWith,
    AvoidOverlapWith,
    PathToExitRequired,
    ServiceReachRequired,
    SameStackPreferred,
    SameFloorRequired,
    TransferAllowedOnlyIf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyConstraintStrength {
    Hard,
    Soft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyConstraintScope {
    Room,
    Space,
    Zone,
    Unit,
    Floor,
    Stack,
    Roof,
    Site,
    Support,
    Residual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyConstraintStatus {
    Satisfied,
    Violated,
    Waived,
    Deferred,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyViolationKind {
    MissingEntryTransition,
    DirectBedroomExposure,
    FragmentedWetCluster,
    MissingDaylightOwnership,
    StructureCompatibilityFailure,
    MissingSupportAffinity,
    OutdoorConflictCrossing,
    RoofEgressFailure,
    VerticalContinuityBreak,
    ResidualUnresolved,
    ConstraintViolation,
    GeometryOverlap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyViolationSeverity {
    Warning,
    Error,
    ManualReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyRepairState {
    Detected,
    Scoped,
    SnapshotTaken,
    Repairing,
    Validated,
    Committed,
    RolledBack,
    RoutedUnclear,
}

#[derive(Debug, Clone)]
pub struct TopologyIdentityRecord {
    pub stable_id: String,
    pub runtime_id: String,
    pub entity_kind: TopologyEntityKind,
    pub lineage_parent_stable_id: Option<String>,
    pub version: u32,
    pub family_id: Option<String>,
    pub level_index: Option<u32>,
    pub representative_source_stable_id: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TopologyProvenanceRecord {
    pub provenance_id: String,
    pub entity_runtime_id: String,
    pub entity_kind: TopologyEntityKind,
    pub source_kind: TopologyProvenanceKind,
    pub source_ref: String,
    pub pass_label: String,
    pub family_id: Option<String>,
    pub level_index: Option<u32>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TopologyConstraint {
    pub constraint_id: String,
    pub scope: TopologyConstraintScope,
    pub constraint_kind: TopologyConstraintKind,
    pub strength: TopologyConstraintStrength,
    pub subject_refs: Vec<String>,
    pub target_refs: Vec<String>,
    pub gate_condition: Option<String>,
    pub status: TopologyConstraintStatus,
    pub evidence_refs: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TopologyConstraintGraph {
    pub graph_id: String,
    pub scope_kind: String,
    pub constraints: Vec<TopologyConstraint>,
    pub notes: Vec<String>,
}

impl TopologyConstraintGraph {
    pub fn empty(graph_id: impl Into<String>, scope_kind: impl Into<String>) -> Self {
        Self {
            graph_id: graph_id.into(),
            scope_kind: scope_kind.into(),
            constraints: Vec::new(),
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TopologyViolation {
    pub violation_id: String,
    pub violation_kind: TopologyViolationKind,
    pub severity: TopologyViolationSeverity,
    pub level_index: Option<u32>,
    pub family_id: Option<String>,
    pub owner_refs: Vec<String>,
    pub node_refs: Vec<String>,
    pub edge_refs: Vec<String>,
    pub geometry_refs: Vec<String>,
    pub source_pass: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DirtyRegionOwnership {
    pub dirty_region_id: String,
    pub level_index: Option<u32>,
    pub family_id: Option<String>,
    pub owner_ids: Vec<String>,
    pub dirty_node_ids: Vec<String>,
    pub dirty_edge_ids: Vec<String>,
    pub geometry_refs: Vec<String>,
    pub area_ratio: f64,
    pub violation_ids: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PreservedTopologyBoundary {
    pub boundary_id: String,
    pub preserved_node_ids: Vec<String>,
    pub preserved_edge_ids: Vec<String>,
    pub preserved_owner_ids: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TopologySnapshot {
    pub snapshot_id: String,
    pub graph_id: String,
    pub level_index: Option<u32>,
    pub node_ids: Vec<String>,
    pub edge_ids: Vec<String>,
    pub owner_ids: Vec<String>,
    pub geometry_binding_ids: Vec<String>,
    pub identity_refs: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LocalRepairScope {
    pub scope_id: String,
    pub dirty_region_id: String,
    pub mutable_node_ids: Vec<String>,
    pub mutable_edge_ids: Vec<String>,
    pub mutable_owner_ids: Vec<String>,
    pub preserved_boundary: PreservedTopologyBoundary,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TopologyRepairTransition {
    pub transition_id: String,
    pub from_state: Option<TopologyRepairState>,
    pub to_state: TopologyRepairState,
    pub reason: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TopologyRepairTransaction {
    pub transaction_id: String,
    pub violation_ids: Vec<String>,
    pub dirty_region_id: Option<String>,
    pub scope_id: Option<String>,
    pub snapshot_id: Option<String>,
    pub state: TopologyRepairState,
    pub invalidated_stable_ids: Vec<String>,
    pub preserved_boundary: Option<PreservedTopologyBoundary>,
    pub transitions: Vec<TopologyRepairTransition>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TopologyRepairJournal {
    pub journal_id: String,
    pub violations: Vec<TopologyViolation>,
    pub dirty_regions: Vec<DirtyRegionOwnership>,
    pub repair_scopes: Vec<LocalRepairScope>,
    pub snapshots: Vec<TopologySnapshot>,
    pub transactions: Vec<TopologyRepairTransaction>,
    pub notes: Vec<String>,
}

impl TopologyRepairJournal {
    pub fn empty(journal_id: impl Into<String>) -> Self {
        Self {
            journal_id: journal_id.into(),
            violations: Vec::new(),
            dirty_regions: Vec::new(),
            repair_scopes: Vec::new(),
            snapshots: Vec::new(),
            transactions: Vec::new(),
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnitTopologyNode {
    pub node_id: String,
    pub semantic_kind: UnitRoomSemanticKind,
    pub label: String,
    pub partition: UnitRoomPartition,
    pub required: bool,
    pub daylight_required: bool,
    pub frontage_priority: u8,
    pub wet_cluster_id: Option<String>,
    pub structure_profile: Vec<String>,
    pub vertical_dependency_ids: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UnitTopologyEdge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub edge_kind: UnitTopologyEdgeKind,
    pub directed: bool,
    pub required: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RoomAccessChain {
    pub chain_id: String,
    pub room_node_ids: Vec<String>,
    pub direct_bedroom_exposure: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RoomDaylightOwnership {
    pub room_node_id: String,
    pub ownership_mode: String,
    pub inherited_from_room_node_id: Option<String>,
    pub satisfied: bool,
    pub frontage_priority: u8,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RoomStructureCompatibility {
    pub room_node_id: String,
    pub frontage_fit: bool,
    pub wet_stack_fit: bool,
    pub beam_column_tolerance: String,
    pub bay_suitability: String,
    pub overall_fit: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WetZoneCluster {
    pub cluster_id: String,
    pub member_node_ids: Vec<String>,
    pub legal_secondary_cluster: bool,
    pub compactness_score: f64,
    pub fragmented: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UnitTopologyGraph {
    pub graph_id: String,
    pub unit_type: String,
    pub fallback_mode: Option<String>,
    pub nodes: Vec<UnitTopologyNode>,
    pub edges: Vec<UnitTopologyEdge>,
    pub contract_graph: TopologyConstraintGraph,
    pub access_chains: Vec<RoomAccessChain>,
    pub daylight_ownerships: Vec<RoomDaylightOwnership>,
    pub structure_compatibility: Vec<RoomStructureCompatibility>,
    pub wet_zone_clusters: Vec<WetZoneCluster>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UnitTopologyValidation {
    pub overall_status: String,
    pub living_chain_pass: bool,
    pub privacy_pass: bool,
    pub bath_access_pass: bool,
    pub entry_sequence_pass: bool,
    pub partition_pass: bool,
    pub wet_zone_cluster_pass: bool,
    pub daylight_pass: bool,
    pub structure_pass: bool,
    pub internal_circulation_pass: bool,
    pub direct_bedroom_exposure: bool,
    pub hard_violation_count: usize,
    pub soft_violation_count: usize,
    pub violated_constraint_ids: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UnitGeometryEmbedding {
    pub embedding_id: String,
    pub pattern: String,
    pub topology_first: bool,
    pub fallback_used: bool,
    pub slot_count: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UnitGeometryValidation {
    pub overall_status: String,
    pub exact_non_overlap_pass: bool,
    pub degenerate_room_count: usize,
    pub missing_habitable_opening_count: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportProgramKind {
    EntryLobby,
    MailPackage,
    Bicycle,
    CommonLaundry,
    IndoorAmenity,
    ElevatorMachineRoom,
    WheelchairLift,
    MainElectricalRoom,
    CustomerStationRoom,
    GeneratorRoom,
    AtsRoom,
    EmergencyLightingInverterRoom,
    SolarBatteryUpsRoom,
    MechanicalAhuRoom,
    MechanicalVentilationRiser,
    MpoeRoom,
    DasRoom,
    SubElectricalRoom,
    DomesticWaterBoosterRoom,
    CisternWaterStorageRoom,
    BackflowPreventerRoom,
    CentralWaterHeatingRoom,
    GraywaterSystemRoom,
    PlumbingRiserShaft,
    WaterPrvCloset,
    GasUtilityRoom,
    WaterFiltration,
    GreaseInterceptorRoom,
    FirePumpRoom,
    FireControl,
    SprinklerRiserClosets,
    RainwaterHarvesting,
    TrashJanitorBoh,
    LoadingSupport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyAffinityPolicy {
    Required,
    Preferred,
    Avoid,
    NotRelevant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundFloorPolicy {
    Required,
    Preferred,
    Forbidden,
    Allowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaftAlignmentPolicy {
    Required,
    Preferred,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloorDistributionPolicy {
    SingleInstance,
    GroundOnly,
    DistributedByFloor,
    PodiumPlusTower,
    SplitWhenThresholdExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerimeterPolicy {
    Required,
    Preferred,
    Avoid,
    Either,
}

#[derive(Debug, Clone)]
pub struct SupportTopologyProfile {
    pub program_kind: SupportProgramKind,
    pub near_core_policy: TopologyAffinityPolicy,
    pub near_loading_policy: TopologyAffinityPolicy,
    pub ground_floor_policy: GroundFloorPolicy,
    pub shaft_alignment_policy: ShaftAlignmentPolicy,
    pub distribution_policy: FloorDistributionPolicy,
    pub perimeter_policy: PerimeterPolicy,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SupportTopologyNode {
    pub support_id: String,
    pub program_kind: SupportProgramKind,
    pub display_name: String,
    pub required_area_sf: f64,
    pub assigned_space_ids: Vec<String>,
    pub preferred_level_indices: Vec<u32>,
    pub required_level_indices: Vec<u32>,
    pub profile: SupportTopologyProfile,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SupportTopologyState {
    pub state_id: String,
    pub nodes: Vec<SupportTopologyNode>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutdoorSiteNodeKind {
    ArrivalEntry,
    Forecourt,
    PedestrianRoute,
    AccessibleRoute,
    ServiceAccess,
    FireAccess,
    ParkingWalk,
    DropoffPocket,
    LoadingZone,
    LandscapeZone,
    OpenSpaceZone,
    PrivacyBuffer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutdoorSiteEdgeKind {
    PedestrianFlow,
    AccessibleFlow,
    ServiceFlow,
    FireFlow,
    ParkingWalkFlow,
    SeparationBuffer,
    ConflictCrossing,
    LobbyLink,
}

#[derive(Debug, Clone)]
pub struct OutdoorSiteNode {
    pub node_id: String,
    pub node_kind: OutdoorSiteNodeKind,
    pub area_sf: f64,
    pub linked_summary_kind: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OutdoorSiteEdge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub edge_kind: OutdoorSiteEdgeKind,
    pub required: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OutdoorSiteTopologyGraph {
    pub graph_id: String,
    pub nodes: Vec<OutdoorSiteNode>,
    pub edges: Vec<OutdoorSiteEdge>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeTopologyFamily {
    Bar,
    L,
    U,
    O,
    H,
    Tower,
    X,
    Cluster,
    FreeForm,
    PerimeterPartial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeTopologyMotifKind {
    LinearBar,
    Elbow,
    Courtyard,
    CourtyardMouth,
    ClosedRing,
    RingBreak,
    WingBridge,
    NotchThroat,
    BranchLobe,
    OpenEdge,
    ClusterAdjacency,
    FreeFormCorridor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeTopologyNodeKind {
    Spine,
    Hub,
    Wing,
    Lobe,
    Courtyard,
    CourtyardMouth,
    Ring,
    RingBreak,
    Bridge,
    NotchThroat,
    OpenEdge,
    PerimeterArc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeTopologyEdgeKind {
    SpineContinuation,
    HubAttachment,
    WingAttachment,
    CourtyardBoundary,
    CourtyardMouthAccess,
    RingClosure,
    RingBreak,
    BridgeConnector,
    ClusterAdjacency,
    PerimeterOpening,
}

#[derive(Debug, Clone)]
pub struct ShapeTopologyMotif {
    pub motif_id: String,
    pub motif_kind: ShapeTopologyMotifKind,
    pub quality_score: f64,
    pub required: bool,
    pub realized: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ShapeTopologyNode {
    pub node_id: String,
    pub node_kind: ShapeTopologyNodeKind,
    pub polygon: Vec<Point2>,
    pub quality_score: f64,
    pub required: bool,
    pub realized: bool,
    pub motif_ids: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ShapeTopologyEdge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub edge_kind: ShapeTopologyEdgeKind,
    pub quality_score: f64,
    pub required: bool,
    pub realized: bool,
    pub motif_ids: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ShapeTopologyGrammar {
    pub grammar_id: String,
    pub level_index: u32,
    pub family_id: String,
    pub declared_family: ShapeTopologyFamily,
    pub corridor_family: String,
    pub fallback_used: bool,
    pub manual_review_required: bool,
    pub motifs: Vec<ShapeTopologyMotif>,
    pub nodes: Vec<ShapeTopologyNode>,
    pub edges: Vec<ShapeTopologyEdge>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoofZoneKind {
    OccupiedAmenity,
    PoolBasin,
    PoolDeck,
    MechanicalEquipment,
    MechanicalExclusion,
    Overrun,
    AccessibleEgressPath,
    MaintenancePath,
    DrainageBasin,
    DrainageCorridor,
    ScreeningBuffer,
    ResidualReusable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoofTopologyEdgeKind {
    OccupancyAdjacency,
    Exclusion,
    EgressConnectivity,
    ServiceSeparation,
    DrainageDependency,
    ScreeningBoundary,
    MaintenanceAccess,
}

#[derive(Debug, Clone)]
pub struct RoofTopologyNode {
    pub node_id: String,
    pub level_index: u32,
    pub zone_kind: RoofZoneKind,
    pub area_sf: f64,
    pub occupied: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RoofTopologyEdge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub edge_kind: RoofTopologyEdgeKind,
    pub required: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RoofTopologyGraph {
    pub graph_id: String,
    pub level_index: u32,
    pub nodes: Vec<RoofTopologyNode>,
    pub edges: Vec<RoofTopologyEdge>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalNodeKind {
    Core,
    ElevatorBank,
    FreightElevator,
    EgressStair,
    ScissorStairPair,
    WetStack,
    PlumbingRiser,
    ElectricalRiser,
    VentilationRiser,
    TransferLobby,
    PodiumInterface,
    TowerBranch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalTopologyEdgeKind {
    Continuity,
    ServiceRiserContinuity,
    WetStackContinuity,
    TransferLink,
    SkipStopLink,
    DuplexLink,
    ScissorShare,
    RoomDependency,
}

#[derive(Debug, Clone)]
pub struct VerticalTopologyNode {
    pub node_id: String,
    pub node_kind: VerticalNodeKind,
    pub legacy_group_name: String,
    pub served_floor_min: u32,
    pub served_floor_max: u32,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VerticalTopologyEdge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub edge_kind: VerticalTopologyEdgeKind,
    pub required: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VerticalContinuityGroup {
    pub group_id: String,
    pub continuity_kind: String,
    pub member_node_ids: Vec<String>,
    pub served_levels: Vec<u32>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VerticalStackOwnership {
    pub ownership_id: String,
    pub stack_id: String,
    pub owner_ref: String,
    pub dependent_room_ids: Vec<String>,
    pub dependent_space_ids: Vec<String>,
    pub level_indices: Vec<u32>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VerticalRoomDependency {
    pub dependency_id: String,
    pub room_ref: String,
    pub stack_node_ids: Vec<String>,
    pub dependency_kind: String,
    pub required: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VerticalTransferTopology {
    pub transfer_id: String,
    pub level_index: u32,
    pub from_node_ids: Vec<String>,
    pub to_node_ids: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VerticalTopologyState {
    pub state_id: String,
    pub nodes: Vec<VerticalTopologyNode>,
    pub edges: Vec<VerticalTopologyEdge>,
    pub continuity_groups: Vec<VerticalContinuityGroup>,
    pub stack_ownerships: Vec<VerticalStackOwnership>,
    pub room_dependencies: Vec<VerticalRoomDependency>,
    pub transfer_topologies: Vec<VerticalTransferTopology>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualKind {
    LeftoverWedge,
    NotchPocket,
    ServiceNiche,
    UnusableSliver,
    ConvertibleReserve,
    FutureShaftPocket,
    OutdoorLeftoverStrip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualUsabilityClass {
    Discardable,
    RepairableUnresolved,
    ServiceConvertible,
    ShaftReservable,
    OutdoorBufferRetainable,
    ManualReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualTopologyEdgeKind {
    ToCoreAdjacency,
    ToServiceBandAdjacency,
    ToUnitFrontageAdjacency,
    ToOutdoorBoundaryAdjacency,
    Mergeable,
    ReserveContinuity,
    ConversionCandidate,
}

#[derive(Debug, Clone)]
pub struct ResidualTopologyNode {
    pub residual_id: String,
    pub kind: ResidualKind,
    pub owner_scope: TopologyOwnershipScopeKind,
    pub level_index: Option<u32>,
    pub family_id: Option<String>,
    pub area_sf: f64,
    pub geometry_ref: String,
    pub usability_class: ResidualUsabilityClass,
    pub converted_to_space_id: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResidualTopologyEdge {
    pub edge_id: String,
    pub from_residual_id: String,
    pub to_ref: String,
    pub edge_kind: ResidualTopologyEdgeKind,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResidualTopologyGraph {
    pub graph_id: String,
    pub level_index: Option<u32>,
    pub family_id: Option<String>,
    pub nodes: Vec<ResidualTopologyNode>,
    pub edges: Vec<ResidualTopologyEdge>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FloorVolumetricTopologySummary {
    pub level_index: u32,
    pub family_id: String,
    pub vertical_void_count: usize,
    pub double_height_count: usize,
    pub mezzanine_count: usize,
    pub shaft_reserve_count: usize,
    pub slab_opening_count: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VolumetricTopologyEvaluation {
    pub overall_status: crate::PassWarnFailStatus,
    pub vertical_void_count: usize,
    pub double_height_count: usize,
    pub mezzanine_count: usize,
    pub duplex_coupling_count: usize,
    pub skip_stop_coupling_count: usize,
    pub shaft_reserve_continuity_pass_ratio: f64,
    pub slab_opening_conflict_count: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConceptEnvelope {
    pub building_shape: BuildingShape,
    pub site_area_sf: f64,
    pub site_perimeter_ft: f64,
    pub gfa_goal_sf: f64,
    pub footprint_seed_sf: f64,
    pub podium_footprint_sf: f64,
    pub upper_footprint_sf: f64,
    pub shape_diagnostics: ShapeRealizationDiagnostics,
    pub concept_blocks: Vec<ConceptBlock>,
    pub family_block_bindings: Vec<ConceptFamilyBlockBinding>,
}

#[derive(Debug, Clone)]
pub struct LayoutFlexBudget {
    pub max_core_shift_ft: f64,
    pub max_corridor_width_delta_ft: f64,
    pub max_local_depth_adjust_ft: f64,
    pub max_floorplate_area_drift_ratio: f64,
    pub allow_minor_block_edge_regularization: bool,
}

impl Default for LayoutFlexBudget {
    fn default() -> Self {
        Self {
            max_core_shift_ft: 8.0,
            max_corridor_width_delta_ft: 1.0,
            max_local_depth_adjust_ft: 2.0,
            max_floorplate_area_drift_ratio: 0.12,
            allow_minor_block_edge_regularization: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConceptOption {
    pub concept_id: String,
    pub story_count: u32,
    pub below_grade_count: u32,
    pub podium_levels: u32,
    pub building_shape: BuildingShape,
    pub construction_case: ConstructionCase,
    pub shape_case: ShapeCase,
    pub corridor_type_seed: CorridorType,
    pub core_strategy_seed: CoreStrategy,
    pub footprint_seed_sf: f64,
    pub podium_footprint_sf: f64,
    pub upper_footprint_sf: f64,
    pub floor_families: Vec<FloorFamily>,
    pub concept_blocks: Vec<ConceptBlock>,
    pub family_block_bindings: Vec<ConceptFamilyBlockBinding>,
    pub envelope: ConceptEnvelope,
    pub site_plan_bundle: MassingSitePlanBundle,
    pub flex_budget: LayoutFlexBudget,
    pub concept_score: f64,
    pub shape_diagnostics: ShapeRealizationDiagnostics,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DetailDriftMetric {
    pub metric_id: String,
    pub metric_kind: String,
    pub family_id: Option<String>,
    pub level_index: Option<u32>,
    pub subject_ref: Option<String>,
    pub observed_value: f64,
    pub allowed_value: f64,
    pub within_budget: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConceptDeltaRequest {
    pub reason_code: String,
    pub message: String,
    pub requested_changes: Vec<String>,
    pub metric_ids: Vec<String>,
    pub severity: ValidationSeverity,
}

#[derive(Debug, Clone)]
pub struct DetailSolveResult {
    pub selected_candidate: CandidateSolution,
    pub concept_applied: ConceptOption,
    pub delta_requests: Vec<ConceptDeltaRequest>,
    pub drift_metrics: Vec<DetailDriftMetric>,
    pub validation_issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone)]
pub struct EngineState {
    pub input: LayoutInput,
    pub code_pack: JurisdictionCodePack,
    pub assumptions: AssumptionPack,
    pub variables: VariableBook,
    pub normalized: Option<NormalizedInput>,
    pub massing: Option<MassingState>,
    pub program: Option<ProgramState>,
    pub verticals: Option<VerticalState>,
    pub zoning: Option<ZoningState>,
    pub candidate_solutions: Vec<CandidateSolution>,
    pub selected_candidate: Option<CandidateSolution>,
    pub validation_issues: Vec<ValidationIssue>,
}

impl EngineState {
    pub fn new(input: LayoutInput, assumptions: AssumptionPack) -> Self {
        let mut validation_issues = input.validate();
        validation_issues.extend(validate_unit_size_targets_sf(
            &assumptions.unit_size_targets_sf,
        ));
        let code_pack = JurisdictionCodePack::from_input(&input);
        Self {
            input,
            code_pack,
            assumptions,
            variables: VariableBook::default(),
            normalized: None,
            massing: None,
            program: None,
            verticals: None,
            zoning: None,
            candidate_solutions: Vec::new(),
            selected_candidate: None,
            validation_issues,
        }
    }

    pub fn invalidate_from(&mut self, phase: SolvePhase) {
        match phase {
            SolvePhase::InputNormalization => {
                self.normalized = None;
                self.massing = None;
                self.program = None;
                self.verticals = None;
                self.zoning = None;
                self.candidate_solutions.clear();
                self.selected_candidate = None;
            }
            SolvePhase::MassingTargets => {
                self.massing = None;
                self.program = None;
                self.verticals = None;
                self.zoning = None;
                self.candidate_solutions.clear();
                self.selected_candidate = None;
            }
            SolvePhase::ProgramTargets => {
                self.program = None;
                self.verticals = None;
                self.zoning = None;
                self.candidate_solutions.clear();
                self.selected_candidate = None;
            }
            SolvePhase::VerticalSystem => {
                self.verticals = None;
                self.zoning = None;
                self.candidate_solutions.clear();
                self.selected_candidate = None;
            }
            SolvePhase::FloorZoning | SolvePhase::SpaceLayout | SolvePhase::OutputAssembly => {
                self.zoning = None;
                self.candidate_solutions.clear();
                self.selected_candidate = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(lhs: f64, rhs: f64, tol: f64) {
        assert!(
            (lhs - rhs).abs() <= tol,
            "expected {lhs:.6} ~= {rhs:.6} within {tol:.6}"
        );
    }

    #[test]
    fn preliminary_support_staff_ratio_matches_workbook_profiles() {
        let mut affordable = AssumptionPack::default();
        affordable.preliminary_area.default_affordable_support_profile = true;
        approx_eq(
            preliminary_support_staff_ratio(10, 200_000.0, &affordable),
            0.05458710114423372,
            1.0e-6,
        );

        let mut market = affordable.clone();
        market.preliminary_area.default_affordable_support_profile = false;
        approx_eq(
            preliminary_support_staff_ratio(10, 200_000.0, &market),
            0.0498239762562855,
            1.0e-6,
        );
    }

    #[test]
    fn preliminary_gfa_inverse_round_trips_dwelling_unit_seed() {
        let assumptions = AssumptionPack::default();
        let mut state = EngineState::new(crate::sample_repo_integration_input().to_core(), assumptions.clone());
        crate::layout_massing::solve_input_normalization(&mut state);
        let normalized = state.normalized.as_ref().unwrap();
        let mix = normalized.unit_mix_seed.clone().normalized();
        let target_units = 140;
        let gfa_sf = preliminary_gfa_from_dwelling_units(
            target_units,
            0.0,
            10,
            &mix,
            normalized,
            &assumptions,
        );
        let recovered_units = preliminary_dwelling_units_from_gfa(
            gfa_sf,
            0.0,
            10,
            &mix,
            normalized,
            &assumptions,
        );

        assert!(gfa_sf > 0.0);
        assert!((recovered_units as i32 - target_units as i32).abs() <= 1);
    }

    #[test]
    fn workbook_support_formula_anchors_stay_stable() {
        let assumptions = AssumptionPack::default();

        approx_eq(bicycle_room_area_sf(140, &assumptions), 1925.0, 1.0e-6);
        approx_eq(bicycle_repair_area_sf(140, &assumptions), 100.0, 1.0e-6);
        approx_eq(bicycle_repair_area_sf(1000, &assumptions), 200.0, 1.0e-6);
        approx_eq(common_laundry_area_sf(140, 0.0, &assumptions), 521.0, 1.0e-6);
        approx_eq(entry_lobby_area_sf(10, 140, &assumptions), 1680.0, 1.0e-6);
        approx_eq(entry_wind_lobby_area_sf(10, 140, &assumptions), 0.0, 1.0e-6);
        approx_eq(general_storage_area_sf(140, &assumptions), 200.0, 1.0e-6);
        approx_eq(janitor_closet_area_sf(140, &assumptions), 144.0, 1.0e-6);
        approx_eq(parcel_storage_area_sf(140, &assumptions), 100.0, 1.0e-6);
        approx_eq(cold_storage_delivery_area_sf(140, &assumptions), 100.0, 1.0e-6);
        approx_eq(leasing_office_area_sf(140, &assumptions), 100.0, 1.0e-6);
        approx_eq(manager_office_area_sf(140, &assumptions), 100.0, 1.0e-6);
        let mut market_assumptions = assumptions.clone();
        market_assumptions.preliminary_area.default_affordable_support_profile = false;
        approx_eq(manager_office_area_sf(140, &market_assumptions), 0.0, 1.0e-6);
        approx_eq(
            entry_wind_lobby_count(10, 140, &market_assumptions),
            1.1333333333333333,
            1.0e-6,
        );
        approx_eq(
            entry_wind_lobby_area_sf(10, 140, &market_assumptions),
            72.53333333333333,
            1.0e-6,
        );
        approx_eq(cctv_room_area_sf(140, &assumptions), 60.0, 1.0e-6);
        approx_eq(staff_break_room_area_sf(140, &assumptions), 0.0, 1.0e-6);
        let mut staff_break_room_assumptions = assumptions.clone();
        staff_break_room_assumptions.support.staff_break_room_enabled = true;
        assert_eq!(staff_break_room_count(140, &staff_break_room_assumptions), 0);
        approx_eq(
            staff_break_room_area_sf(1000, &staff_break_room_assumptions),
            144.0,
            1.0e-6,
        );
        approx_eq(staff_locker_showers_area_sf(3, &assumptions), 549.0, 1.0e-6);
        approx_eq(staff_restroom_area_sf(3, true, &assumptions), 61.2, 1.0e-6);
        let sample_input = crate::sample_repo_integration_input().to_core();
        let mut sample_state = EngineState::new(sample_input, assumptions.clone());
        crate::layout_massing::solve_input_normalization(&mut sample_state);
        let sample_normalized = sample_state.normalized.as_ref().unwrap();
        approx_eq(
            resident_restroom_area_sf(
                sample_normalized,
                indoor_amenity_target_sf(sample_normalized, 140, &assumptions),
                &assumptions,
            ),
            0.0,
            1.0e-6,
        );
        let mut club_room_input = crate::sample_repo_integration_input();
        club_room_input.levels.count = 10;
        club_room_input.targets.dwelling_units_cap = Some(140);
        club_room_input.amenities.include.clear();
        club_room_input.amenities.include.push("club_room".to_string());
        let club_room_core = club_room_input.to_core();
        let mut club_room_state = EngineState::new(club_room_core, assumptions.clone());
        crate::layout_massing::solve_input_normalization(&mut club_room_state);
        let club_room_normalized = club_room_state.normalized.as_ref().unwrap();
        approx_eq(
            resident_restroom_area_sf(
                club_room_normalized,
                indoor_amenity_target_sf(club_room_normalized, 140, &assumptions),
                &assumptions,
            ),
            169.2,
            1.0e-6,
        );
        let mut fan_room_amenity_input = crate::sample_repo_integration_input();
        fan_room_amenity_input.levels.count = 10;
        fan_room_amenity_input.targets.dwelling_units_cap = Some(140);
        fan_room_amenity_input.amenities.indoor_target_sf = Some(5400.0);
        fan_room_amenity_input
            .amenities
            .include
            .extend(["club_room", "fitness", "cowork"].iter().map(|s| s.to_string()));
        let fan_room_amenity_core = fan_room_amenity_input.to_core();
        let mut fan_room_amenity_state =
            EngineState::new(fan_room_amenity_core, assumptions.clone());
        crate::layout_massing::solve_input_normalization(&mut fan_room_amenity_state);
        let fan_room_amenity_normalized = fan_room_amenity_state.normalized.as_ref().unwrap();
        approx_eq(
            fan_room_ahu_indoor_amenity_supply_air_cfm(
                fan_room_amenity_normalized,
                5400.0,
                &assumptions,
            ),
            2689.1128160582105,
            1.0e-6,
        );
        let mut amenity_alias_input = crate::sample_repo_integration_input();
        amenity_alias_input.levels.count = 10;
        amenity_alias_input.targets.dwelling_units_cap = Some(140);
        amenity_alias_input.amenities.indoor_target_sf = Some(2878.8157869809525);
        amenity_alias_input.amenities.include.extend(
            [
                "business_center_coworking",
                "bar_cafe_nook",
                "concierge_desk",
                "massage_treatment_room",
                "sauna_steam_room",
                "spa_wellness_center",
                "theater_screening_room",
            ]
            .iter()
            .map(|s| s.to_string()),
        );
        let amenity_alias_core = amenity_alias_input.to_core();
        let mut amenity_alias_state = EngineState::new(amenity_alias_core, assumptions.clone());
        crate::layout_massing::solve_input_normalization(&mut amenity_alias_state);
        let amenity_alias_normalized = amenity_alias_state.normalized.as_ref().unwrap();
        let detailed_rows = indoor_amenity_detailed_program_areas(
            amenity_alias_normalized,
            2878.8157869809525,
            &assumptions,
        );
        for expected_name in [
            "Bar / Café Nook",
            "Concierge Desk",
            "Massage / Treatment Room",
            "Sauna / Steam Room",
            "Spa / Wellness Center",
            "Theater / Screening Room",
        ] {
            assert!(detailed_rows
                .iter()
                .any(|(space_name, area_sf)| space_name == expected_name && *area_sf > EPS));
        }
        let mut expanded_restroom_input = crate::sample_repo_integration_input();
        expanded_restroom_input.levels.count = 10;
        expanded_restroom_input.targets.dwelling_units_cap = Some(140);
        expanded_restroom_input.amenities.indoor_target_sf = Some(8500.0);
        expanded_restroom_input.amenities.include.clear();
        expanded_restroom_input.amenities.include.extend(
            [
                "club_room",
                "dog_washing_room",
                "bar_cafe_nook",
                "concierge_desk",
                "massage_treatment_room",
                "spa_wellness_center",
                "sauna_steam_room",
                "theater_screening_room",
            ]
            .iter()
            .map(|s| s.to_string()),
        );
        let expanded_restroom_core = expanded_restroom_input.to_core();
        let mut expanded_restroom_state =
            EngineState::new(expanded_restroom_core, assumptions.clone());
        crate::layout_massing::solve_input_normalization(&mut expanded_restroom_state);
        let expanded_restroom_normalized = expanded_restroom_state.normalized.as_ref().unwrap();
        let expanded_restroom_area_sf =
            resident_restroom_area_sf(expanded_restroom_normalized, 8500.0, &assumptions);
        assert!(expanded_restroom_area_sf > EPS);
        assert!(
            expanded_restroom_area_sf
                >= resident_restroom_area_sf(
                    club_room_normalized,
                    indoor_amenity_target_sf(club_room_normalized, 140, &assumptions),
                    &assumptions,
                )
        );
        let mut kitchen_input = crate::sample_repo_integration_input();
        kitchen_input.levels.count = 10;
        kitchen_input.targets.dwelling_units_cap = Some(140);
        kitchen_input
            .amenities
            .include
            .push("Commercial Kitchen".to_string());
        let kitchen_core = kitchen_input.to_core();
        let mut kitchen_state = EngineState::new(kitchen_core, assumptions.clone());
        crate::layout_massing::solve_input_normalization(&mut kitchen_state);
        let kitchen_normalized = kitchen_state.normalized.as_ref().unwrap();
        approx_eq(
            commercial_kitchen_shaft_area_sf(kitchen_normalized, 140, 10, &assumptions),
            108.125,
            1.0e-6,
        );
        assert_eq!(mpoe_room_count(140, &assumptions), 1);
        approx_eq(mpoe_room_area_sf(140, &assumptions), 100.0, 1.0e-6);
        assert_eq!(idf_closet_count(140, 10, &assumptions), 9);
        approx_eq(idf_closets_area_sf(140, 10, &assumptions), 216.0, 1.0e-6);
        approx_eq(idf_closets_area_sf(140, 3, &assumptions), 0.0, 1.0e-6);
        approx_eq(
            elevator_machine_room_roof_area_sf(10, 1, 0, &assumptions),
            133.33333333333334,
            1.0e-6,
        );
        approx_eq(
            elevator_machine_room_roof_area_sf(4, 1, 0, &assumptions),
            63.157894736842103,
            1.0e-6,
        );
        let mut interior_machine_room_assumptions = assumptions.clone();
        interior_machine_room_assumptions.vertical.passenger_machine_room_on_roof = false;
        approx_eq(
            elevator_machine_room_interior_area_sf(10, 1, 0, &interior_machine_room_assumptions),
            266.66666666666669,
            1.0e-6,
        );
        approx_eq(wheelchair_lift_area_sf(1, 140, &assumptions), 0.0, 1.0e-6);
        let mut wheelchair_lift_assumptions = assumptions.clone();
        wheelchair_lift_assumptions.vertical.wheelchair_lift_enabled = true;
        assert_eq!(wheelchair_lift_count(10, 140, &wheelchair_lift_assumptions), 0);
        assert_eq!(wheelchair_lift_count(1, 140, &wheelchair_lift_assumptions), 1);
        approx_eq(
            wheelchair_lift_area_sf(1, 140, &wheelchair_lift_assumptions),
            50.0,
            1.0e-6,
        );
        approx_eq(electrical_elevator_demand_factor(2), 0.95, 1.0e-6);
        approx_eq(electrical_dwelling_unit_diversity_factor(140), 0.23, 1.0e-6);
        approx_eq(
            coarse_total_electrical_load_kva(
                140,
                2,
                0,
                130198.182674735,
                10111.5577373014,
                10905.0,
                2535.2380952381,
                1680.0,
                0.0,
                &assumptions,
            ),
            1277.61626121977,
            1.0e-6,
        );
        approx_eq(
            main_electrical_room_project_area_sf(1277.61626121977, 140, &assumptions),
            320.0,
            1.0e-6,
        );
        approx_eq(
            main_electrical_room_project_area_sf(1277.61626121977, 16, &assumptions),
            0.0,
            1.0e-6,
        );
        approx_eq(
            electrical_customer_station_indoor_area_sf(
                1277.61626121977,
                &assumptions,
            ),
            949.75500980333743,
            1.0e-6,
        );
        approx_eq(
            electrical_utility_infrastructure_exterior_area_sf(1277.61626121977, &assumptions),
            0.0,
            1.0e-6,
        );
        let mut customer_station_exterior_assumptions = assumptions.clone();
        customer_station_exterior_assumptions
            .boh
            .electrical_customer_station_indoor_enabled = false;
        approx_eq(
            electrical_utility_infrastructure_exterior_area_sf(
                1277.61626121977,
                &customer_station_exterior_assumptions,
            ),
            192.76215277777774,
            1.0e-6,
        );
        approx_eq(
            electrical_utility_infrastructure_exterior_area_sf(
                100.0,
                &assumptions,
            ),
            52.712617187499994,
            1.0e-6,
        );
        approx_eq(
            generator_room_required_standby_kva(
                10,
                2,
                0,
                10111.5577373014,
                10905.0,
                2535.2380952381,
                1680.0,
                0.0,
                &assumptions,
            ),
            882.827297316143,
            1.0e-6,
        );
        approx_eq(
            generator_room_area_sf(
                10,
                2,
                0,
                10111.5577373014,
                10905.0,
                2535.2380952381,
                1680.0,
                0.0,
                &assumptions,
            ),
            308.292798611111,
            1.0e-6,
        );
        approx_eq(
            emergency_electrical_running_kva(
                2,
                0,
                10111.5577373014,
                10905.0,
                2535.2380952381,
                1680.0,
                0.0,
                &assumptions,
            ),
            396.255115174205,
            1.0e-6,
        );
        assert_eq!(
            generator_room_count(
                10,
                2,
                0,
                10111.5577373014,
                10905.0,
                2535.2380952381,
                1680.0,
                0.0,
                &assumptions,
            ),
            1
        );
        approx_eq(
            ats_room_area_sf(1277.61626121977, 396.255115174205, 1, true, &assumptions),
            131.909722222222,
            1.0e-6,
        );
        approx_eq(
            emergency_lighting_inverter_room_area_sf(355.255115174205, false, 140, &assumptions),
            490.252058940405,
            1.0e-6,
        );
        approx_eq(
            solar_battery_ups_room_area_sf(
                10,
                140,
                396.255115174205,
                17924.422183651852,
                None,
                &assumptions,
            ),
            615.98600000000022,
            1.0e-6,
        );
        let fan_room_non_residential_cfm = fan_room_ahu_non_residential_supply_air_cfm(
            sample_normalized,
            1680.0,
            10111.55773730142,
            10905.0,
            &[
                SpaceDemandRow {
                    category: "Amenities".to_string(),
                    space_name: "Indoor Amenity".to_string(),
                    qty: 1.0,
                    area_sf: 1680.0,
                },
                SpaceDemandRow {
                    category: "Support Areas".to_string(),
                    space_name: "Bicycle Repair Area".to_string(),
                    qty: 1.0,
                    area_sf: 100.0,
                },
                SpaceDemandRow {
                    category: "Support Areas".to_string(),
                    space_name: "Bicycle Room".to_string(),
                    qty: 1.0,
                    area_sf: 1925.0,
                },
                SpaceDemandRow {
                    category: "Support Areas".to_string(),
                    space_name: "Common Laundry Room".to_string(),
                    qty: 1.0,
                    area_sf: 521.0,
                },
                SpaceDemandRow {
                    category: "Circulation".to_string(),
                    space_name: "Entry Lobby".to_string(),
                    qty: 1.0,
                    area_sf: 1680.0,
                },
                SpaceDemandRow {
                    category: "Support Areas".to_string(),
                    space_name: "Mail Room / Mail Area".to_string(),
                    qty: 1.0,
                    area_sf: 247.89916666666664,
                },
                SpaceDemandRow {
                    category: "Support Areas".to_string(),
                    space_name: "Storage Room - General / Maintenance".to_string(),
                    qty: 1.0,
                    area_sf: 200.0,
                },
                SpaceDemandRow {
                    category: "Support Areas".to_string(),
                    space_name: "Storage Room - Cold Storage Delivery Room".to_string(),
                    qty: 1.0,
                    area_sf: 100.0,
                },
                SpaceDemandRow {
                    category: "Support Areas".to_string(),
                    space_name: "Storage Room - Parcels".to_string(),
                    qty: 1.0,
                    area_sf: 100.0,
                },
                SpaceDemandRow {
                    category: "Support Areas".to_string(),
                    space_name: "Leasing Office".to_string(),
                    qty: 1.0,
                    area_sf: 100.0,
                },
                SpaceDemandRow {
                    category: "Support Areas".to_string(),
                    space_name: "Manager's Office".to_string(),
                    qty: 1.0,
                    area_sf: 100.0,
                },
                SpaceDemandRow {
                    category: "Support Areas".to_string(),
                    space_name: "CCTV / IT / Security Equipment Rooms".to_string(),
                    qty: 1.0,
                    area_sf: 60.0,
                },
            ],
            &assumptions,
        );
        approx_eq(fan_room_non_residential_cfm, 4518.093991933169, 1.0e-6);
        let fan_room_total_supply_air_cfm = fan_room_ahu_total_supply_air_cfm(
            [4, 111, 25, 0],
            130198.182674735,
            fan_room_non_residential_cfm,
            0.0,
            &assumptions,
        );
        approx_eq(fan_room_total_supply_air_cfm, 9639.039472175218, 1.0e-6);
        approx_eq(
            mechanical_ventilation_riser_residential_supply_air_cfm(
                [4, 111, 25, 0],
                130198.182674735,
                &assumptions,
            ),
            5120.9454802420623,
            1.0e-6,
        );
        approx_eq(
            mechanical_ventilation_riser_exhaust_air_cfm([4, 111, 25, 0], &assumptions),
            22250.0,
            1.0e-6,
        );
        let fan_room_area_sf = fan_room_ahu_room_area_sf(
            [4, 111, 25, 0],
            130198.182674735,
            fan_room_non_residential_cfm,
            0.0,
            &assumptions,
        );
        approx_eq(fan_room_area_sf, 116.68818115033516, 1.0e-6);
        approx_eq(
            mechanical_pad_outdoor_area_sf(140, 130198.182674735, 16544.442539934545, &assumptions),
            0.0,
            1.0e-6,
        );
        let mut outdoor_mechanical_assumptions = assumptions.clone();
        outdoor_mechanical_assumptions.boh.mechanical_pad_outdoor_enabled = true;
        approx_eq(
            mechanical_pad_outdoor_area_sf(
                140,
                130198.182674735,
                16544.442539934545,
                &outdoor_mechanical_assumptions,
            ),
            875.49999999999989,
            1.0e-6,
        );
        assert_eq!(
            mechanical_ventilation_riser_count(
                [4, 111, 25, 0],
                140,
                16544.442539934545,
                130198.182674735,
                6221.94548024205,
                &assumptions,
            ),
            1
        );
        approx_eq(
            mechanical_ventilation_riser_area_sf(
                [4, 111, 25, 0],
                140,
                16544.442539934545,
                130198.182674735,
                6221.94548024205,
                &assumptions,
            ),
            167.91417722348808,
            1.0e-6,
        );
        approx_eq(
            domestic_cold_water_fixture_units([4, 111, 25, 0], 10, 4, 9, &assumptions),
            1968.45,
            1.0e-6,
        );
        approx_eq(
            domestic_peak_flow_gpm(1968.45, &assumptions),
            335.26604258889842,
            1.0e-6,
        );
        approx_eq(
            coarse_building_occupant_count([4, 111, 25, 0], &assumptions),
            333.0,
            1.0e-6,
        );
        approx_eq(
            water_filtration_area_sf(333.0, &assumptions),
            42.6483513431921,
            1.0e-6,
        );
        approx_eq(
            grease_interceptor_room_area_sf(&assumptions),
            106.31558307692305,
            1.0e-6,
        );
        approx_eq(
            fire_control_area_sf(10, 179244.22183651853, &assumptions),
            200.0,
            1.0e-6,
        );
        let mut fire_control_rack_assumptions = assumptions.clone();
        fire_control_rack_assumptions.boh.fire_control_equipment_rack_count = 2;
        approx_eq(
            fire_control_area_sf(10, 179244.22183651853, &fire_control_rack_assumptions),
            240.0,
            1.0e-6,
        );
        approx_eq(
            fire_control_area_sf(2, 179244.22183651853, &assumptions),
            0.0,
            1.0e-6,
        );
        approx_eq(
            sprinkler_riser_closet_area_sf(2, &assumptions),
            20.0,
            1.0e-6,
        );
        approx_eq(
            sprinkler_riser_closet_area_sf(10, &assumptions),
            0.0,
            1.0e-6,
        );
        approx_eq(
            domestic_water_booster_pump_room_area_sf(
                10,
                179244.22183651853,
                335.26604258889842,
                &assumptions,
            ),
            156.0,
            1.0e-6,
        );
        approx_eq(
            cistern_water_storage_tank_room_area_sf(335.26604258889842, &assumptions),
            1217.252289478726,
            1.0e-6,
        );
        approx_eq(
            backflow_preventer_room_area_sf(335.26604258889842, &assumptions),
            79.953125,
            1.0e-6,
        );
        approx_eq(
            central_water_heating_room_indoor_area_sf(&assumptions),
            85.062809805365731,
            1.0e-6,
        );
        let mut cwh_outdoor_assumptions = assumptions.clone();
        cwh_outdoor_assumptions.boh.central_water_heating_room_indoor_enabled = false;
        cwh_outdoor_assumptions.boh.central_water_heating_pad_outdoor_enabled = true;
        approx_eq(
            central_water_heating_pad_outdoor_area_sf(&cwh_outdoor_assumptions),
            85.062809805365731,
            1.0e-6,
        );
        cwh_outdoor_assumptions.boh.central_water_heating_room_indoor_enabled = true;
        approx_eq(
            central_water_heating_pad_outdoor_area_sf(&cwh_outdoor_assumptions),
            0.0,
            1.0e-6,
        );
        approx_eq(
            graywater_system_room_area_sf(&assumptions),
            68.906878791271083,
            1.0e-6,
        );
        assert_eq!(
            sub_electrical_room_count(10, 140, 179244.22183651853, &assumptions),
            9
        );
        approx_eq(
            sub_electrical_room_single_area_sf(9, 140, &assumptions),
            37.8,
            1.0e-6,
        );
        approx_eq(
            sub_electrical_rooms_area_sf(10, 9, 140, 179244.22183651853, &assumptions),
            340.2,
            1.0e-6,
        );
        approx_eq(gas_utility_meter_room_area_sf(140, &assumptions), 1456.0, 1.0e-6);
        assert_eq!(gas_utility_meter_count(140, &assumptions), 141);
        approx_eq(
            gas_utility_meter_room_area_sf(140, &assumptions),
            1456.0,
            1.0e-6,
        );
        approx_eq(gas_meter_space_alcove_area_sf(140, &assumptions), 0.0, 1.0e-6);
        let mut gas_alcove_assumptions = assumptions.clone();
        gas_alcove_assumptions.boh.gas_utility_room_enabled = false;
        gas_alcove_assumptions.boh.gas_meter_space_alcove_enabled = true;
        approx_eq(
            gas_meter_space_alcove_area_sf(140, &gas_alcove_assumptions),
            1135.0,
            1.0e-6,
        );
        gas_alcove_assumptions.boh.gas_utility_room_enabled = true;
        approx_eq(
            gas_meter_space_alcove_area_sf(140, &gas_alcove_assumptions),
            0.0,
            1.0e-6,
        );
        approx_eq(rainwater_harvesting_area_sf(&assumptions), 107.1219722336979, 1.0e-6);
        approx_eq(
            plumbing_riser_area_sf(140, 6, &assumptions),
            69.252335957535877,
            1.0e-6,
        );
        approx_eq(plumbing_riser_area_sf(140, 1, &assumptions), 0.0, 1.0e-6);
        approx_eq(water_prv_closet_area_sf(6, &assumptions), 0.0, 1.0e-6);
        approx_eq(
            water_prv_closet_area_sf(10, &assumptions),
            2.7715006246702326,
            1.0e-6,
        );
        approx_eq(fire_pump_room_area_sf(10, &assumptions), 144.0, 1.0e-6);
        approx_eq(
            parking_control_room_area_sf(16, ParkingMode::Podium, &assumptions),
            0.0,
            1.0e-6,
        );
        approx_eq(
            parking_control_room_area_sf(301, ParkingMode::Podium, &assumptions),
            104.16,
            1.0e-6,
        );
        approx_eq(loading_dock_area_sf(140, &assumptions), 250.0, 1.0e-6);
        approx_eq(dumpster_support_area_sf(14.985, &assumptions), 210.7265625, 1.0e-6);
        approx_eq(dumpster_support_area_sf(9.99, &assumptions), 140.484375, 1.0e-6);
        approx_eq(dumpster_support_area_sf(8.325, &assumptions), 117.0703125, 1.0e-6);
        approx_eq(trash_vestibule_area_sf(10, 8, &assumptions), 768.0, 1.0e-6);
        approx_eq(
            outdoor_amenity_circulation_area_sf(2742.0, &assumptions),
            548.4,
            1.0e-6,
        );
    }
}
