/* tslint:disable */
/* eslint-disable */

export class RendererHandle {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    render(): void;
    resize(width: number, height: number): void;
    set_camera(camera: any): void;
    set_scene(scene: any): void;
}

export class RendererInfo {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    readonly adapter_name: string;
    readonly backend: string;
}

export function create_renderer(canvas: HTMLCanvasElement): Promise<RendererHandle>;

export function probe_webgpu(): Promise<RendererInfo>;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_rendererhandle_free: (a: number, b: number) => void;
    readonly __wbg_rendererinfo_free: (a: number, b: number) => void;
    readonly create_renderer: (a: any) => any;
    readonly probe_webgpu: () => any;
    readonly rendererhandle_render: (a: number) => [number, number];
    readonly rendererhandle_resize: (a: number, b: number, c: number) => [number, number];
    readonly rendererhandle_set_camera: (a: number, b: any) => [number, number];
    readonly rendererhandle_set_scene: (a: number, b: any) => [number, number];
    readonly rendererinfo_adapter_name: (a: number) => [number, number];
    readonly rendererinfo_backend: (a: number) => [number, number];
    readonly wasm_bindgen__closure__destroy__h81aa3ba43ebfcc3f: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__ha39c779d5f6f9160: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h241616e5b819c31e: (a: number, b: number, c: any, d: any) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
