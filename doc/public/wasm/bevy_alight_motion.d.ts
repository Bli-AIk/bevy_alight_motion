/* tslint:disable */
/* eslint-disable */

/**
 * Get current frame pixels for video comparison
 * 获取当前帧像素数据用于视频对比
 */
export function get_current_frame_pixels(): Uint8Array;

/**
 * Get current player state as JSON
 * 获取当前播放器状态 (JSON 格式)
 */
export function get_state(): any;

/**
 * Load a project from JavaScript (receives ArrayBuffer bytes)
 * 从 JavaScript 加载项目 (接收 ArrayBuffer 字节)
 *
 * This function:
 * 1. Inserts the project bytes into the memory asset source
 * 2. Triggers the Bevy asset system to load it
 */
export function load_project_from_bytes(data: Uint8Array): boolean;

/**
 * Main entry point for WASM
 */
export function main(): void;

/**
 * Pause the animation
 * 暂停动画
 */
export function pause(): void;

/**
 * Play the animation
 * 播放动画
 */
export function play(): void;

/**
 * Reset to the beginning
 * 重置到开头
 */
export function reset(): void;

/**
 * Seek to a specific frame
 * 跳转到指定帧
 */
export function seek(frame: number): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly get_current_frame_pixels: () => [number, number];
    readonly get_state: () => any;
    readonly load_project_from_bytes: (a: number, b: number) => number;
    readonly main: () => void;
    readonly pause: () => void;
    readonly play: () => void;
    readonly reset: () => void;
    readonly seek: (a: number) => void;
    readonly wasm_bindgen__closure__destroy__h4c35e478110bdf75: (a: number, b: number) => void;
    readonly wasm_bindgen__closure__destroy__h1613bb30f37bb86d: (a: number, b: number) => void;
    readonly wasm_bindgen__closure__destroy__h9556aefdb9118ae1: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h2faf70642f58abaa: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h49f886c41ee9f6ea: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1ab621a0c0d22885: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__heef39789d7589a3f: (a: number, b: number, c: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h9035c83a1ce817db: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h66afc66bc0134213: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_exn_store: (a: number) => void;
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
