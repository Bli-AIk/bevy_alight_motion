/* tslint:disable */
/* eslint-disable */

/**
 * Download runtime logs as a text file
 * 下载运行时日志为文本文件 (兼容移动端)
 */
export function download_logs(): void;

/**
 * Get current frame pixels for video comparison
 * 获取当前帧像素数据用于视频对比
 */
export function get_current_frame_pixels(): Uint8Array;

/**
 * Get logs as string for JavaScript
 * 获取日志字符串供 JavaScript 使用
 */
export function get_logs(): string;

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

/**
 * Start the Bevy application.
 * Must be called AFTER `<canvas id="bevy-canvas">` is visible and has non-zero dimensions.
 * On high-DPI mobile devices, cap `window.devicePixelRatio` from JS before calling this.
 */
export function start_app(): void;

/**
 * WASM module initialization — lightweight, no Bevy.
 * Bevy app is started separately via `start_app()` after the canvas is visible.
 */
export function wasm_init(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly download_logs: () => void;
    readonly get_current_frame_pixels: () => [number, number];
    readonly get_logs: () => [number, number];
    readonly get_state: () => any;
    readonly load_project_from_bytes: (a: number, b: number) => number;
    readonly pause: () => void;
    readonly play: () => void;
    readonly reset: () => void;
    readonly seek: (a: number) => void;
    readonly start_app: () => void;
    readonly wasm_init: () => void;
    readonly wasm_bindgen__closure__destroy__h07aa81d6da615279: (a: number, b: number) => void;
    readonly wasm_bindgen__closure__destroy__h38046e54fe5330f7: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__hf8ac97678b257160: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h58816ab24383e09a: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h2f7f4daf63d4d5ba: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h6bf14f86a9d7779d: (a: number, b: number) => void;
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
