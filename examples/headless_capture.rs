use super::HeadlessResolution;
use bevy::image::TextureFormatPixelInfo;
use bevy::prelude::*;
use bevy::render::Extract;
use bevy::render::graph::CameraDriverLabel;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_graph::{NodeRunError, RenderGraph, RenderGraphContext, RenderLabel};
use bevy::render::render_resource::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, MapMode, PollType,
    TexelCopyBufferInfo, TexelCopyBufferLayout, TextureFormat, TextureUsages,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue};
use bevy::render::{Render, RenderApp, RenderSystems};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

#[derive(Resource)]
pub struct HeadlessCaptureState {
    pub pending_path: Option<PathBuf>,
    pub pending_serial: Option<u64>,
    pub next_serial: u64,
    pub discard_captures: u32,
    pub width: u32,
    pub height: u32,
    pub texture_format: TextureFormat,
}

impl Default for HeadlessCaptureState {
    fn default() -> Self {
        Self {
            pending_path: None,
            pending_serial: None,
            next_serial: 1,
            discard_captures: 0,
            width: 0,
            height: 0,
            texture_format: TextureFormat::Rgba8UnormSrgb,
        }
    }
}

#[derive(Resource)]
struct HeadlessCaptureMainReceiver {
    receiver: Mutex<Receiver<(u64, Vec<u8>)>>,
}

#[derive(Resource, Clone)]
struct HeadlessCaptureRenderSender(Sender<(u64, Vec<u8>)>);

#[derive(Clone, Default, Resource, Deref, DerefMut)]
struct HeadlessImageCopiers(Vec<HeadlessImageCopier>);

#[derive(Clone, Component)]
pub struct HeadlessImageCopier {
    buffer: Buffer,
    enabled: Arc<AtomicBool>,
    serial: Arc<AtomicU64>,
    src_image: Handle<Image>,
}

impl HeadlessImageCopier {
    pub fn new(src_image: Handle<Image>, size: Extent3d, render_device: &RenderDevice) -> Self {
        let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(size.width as usize * 4);
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("headless-comparison-capture-buffer"),
            size: padded_bytes_per_row as u64 * size.height as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            enabled: Arc::new(AtomicBool::new(false)),
            serial: Arc::new(AtomicU64::new(0)),
            src_image,
        }
    }

    pub fn request(&self, serial: u64) {
        self.serial.store(serial, Ordering::Relaxed);
        self.enabled.store(true, Ordering::Relaxed);
    }

    fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    fn serial(&self) -> u64 {
        self.serial.load(Ordering::Relaxed)
    }

    fn finish(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }
}

pub struct HeadlessComparisonCapturePlugin;

impl Plugin for HeadlessComparisonCapturePlugin {
    fn build(&self, app: &mut App) {
        let (sender, receiver) = std::sync::mpsc::channel();
        app.insert_resource(HeadlessCaptureMainReceiver {
            receiver: Mutex::new(receiver),
        })
        .init_resource::<HeadlessCaptureState>()
        .add_systems(Update, flush_headless_capture_to_disk);

        let render_app = app.sub_app_mut(RenderApp);
        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(HeadlessCaptureCopyLabel, HeadlessCaptureCopyNode);
        graph.add_node_edge(CameraDriverLabel, HeadlessCaptureCopyLabel);

        render_app
            .insert_resource(HeadlessCaptureRenderSender(sender))
            .add_systems(
                bevy::render::ExtractSchedule,
                extract_headless_image_copiers,
            )
            .add_systems(
                Render,
                receive_headless_capture_from_buffer.after(RenderSystems::Render),
            );
    }
}

pub fn setup_headless_capture(
    commands: &mut Commands,
    render_device: &RenderDevice,
    render_target: Handle<Image>,
    headless_res: &HeadlessResolution,
    state: &mut HeadlessCaptureState,
) {
    let width = headless_res.0.x as u32;
    let height = headless_res.0.y as u32;
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    state.width = width;
    state.height = height;
    state.texture_format = TextureFormat::Rgba8UnormSrgb;
    commands.spawn(HeadlessImageCopier::new(render_target, size, render_device));
}

fn flush_headless_capture_to_disk(
    receiver: Res<HeadlessCaptureMainReceiver>,
    mut state: ResMut<HeadlessCaptureState>,
) {
    let Ok(receiver) = receiver.receiver.lock() else {
        return;
    };

    let mut latest: Option<Vec<u8>> = None;
    while let Ok((serial, data)) = receiver.try_recv() {
        if state.discard_captures > 0 {
            state.discard_captures -= 1;
            continue;
        }
        if state.pending_serial == Some(serial) {
            latest = Some(data);
        }
    }

    let Some(data) = latest else {
        return;
    };

    let Some(path) = state.pending_path.take() else {
        return;
    };
    state.pending_serial = None;

    let row_bytes = state.width as usize * state.texture_format.pixel_size().unwrap();
    let aligned_row_bytes = RenderDevice::align_copy_bytes_per_row(row_bytes);
    let trimmed = if row_bytes == aligned_row_bytes {
        data
    } else {
        data.chunks(aligned_row_bytes)
            .take(state.height as usize)
            .flat_map(|row| row[..row_bytes.min(row.len())].iter().copied())
            .collect()
    };

    let Some(image) = image::RgbaImage::from_raw(state.width, state.height, trimmed) else {
        error!(
            "Failed to build RGBA image for headless capture {}x{}",
            state.width, state.height
        );
        return;
    };

    if let Err(err) = image.save(&path) {
        error!(
            "Failed to save headless comparison shot {}: {err}",
            path.display()
        );
    }
}

fn extract_headless_image_copiers(
    mut commands: Commands,
    image_copiers: Extract<Query<&HeadlessImageCopier>>,
) {
    commands.insert_resource(HeadlessImageCopiers(
        image_copiers.iter().cloned().collect::<Vec<_>>(),
    ));
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, RenderLabel)]
struct HeadlessCaptureCopyLabel;

#[derive(Default)]
struct HeadlessCaptureCopyNode;

impl bevy::render::render_graph::Node for HeadlessCaptureCopyNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let image_copiers = world.resource::<HeadlessImageCopiers>();
        let gpu_images = world.resource::<RenderAssets<bevy::render::texture::GpuImage>>();
        let render_queue = world.resource::<RenderQueue>();

        for image_copier in image_copiers.iter() {
            if !image_copier.enabled() {
                continue;
            }

            let Some(src_image) = gpu_images.get(&image_copier.src_image) else {
                continue;
            };

            let mut encoder =
                render_context
                    .render_device()
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("headless-comparison-copy-encoder"),
                    });

            let block_dimensions = src_image.texture_format.block_dimensions();
            let block_size = src_image.texture_format.block_copy_size(None).unwrap();
            let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
                (src_image.size.width as usize / block_dimensions.0 as usize) * block_size as usize,
            );

            encoder.copy_texture_to_buffer(
                src_image.texture.as_image_copy(),
                TexelCopyBufferInfo {
                    buffer: &image_copier.buffer,
                    layout: TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            std::num::NonZeroU32::new(padded_bytes_per_row as u32)
                                .unwrap()
                                .into(),
                        ),
                        rows_per_image: None,
                    },
                },
                src_image.size,
            );

            render_queue.submit(std::iter::once(encoder.finish()));
        }

        Ok(())
    }
}

fn receive_headless_capture_from_buffer(
    image_copiers: Res<HeadlessImageCopiers>,
    render_device: Res<RenderDevice>,
    sender: Res<HeadlessCaptureRenderSender>,
) {
    for image_copier in image_copiers.iter() {
        if !image_copier.enabled() {
            continue;
        }

        let buffer_slice = image_copier.buffer.slice(..);
        let (signal_tx, signal_rx) = std::sync::mpsc::sync_channel(1);
        buffer_slice.map_async(MapMode::Read, move |result| match result {
            Ok(()) => {
                let _ = signal_tx.send(());
            }
            Err(err) => panic!("Failed to map headless capture buffer: {err}"),
        });

        render_device
            .poll(PollType::wait_indefinitely())
            .expect("Failed to poll headless comparison capture");
        signal_rx
            .recv()
            .expect("Failed to wait for headless capture map_async");

        let serial = image_copier.serial();
        let data = buffer_slice.get_mapped_range().to_vec();
        image_copier.buffer.unmap();
        image_copier.finish();
        let _ = sender.0.send((serial, data));
    }
}

pub fn mark_render_target_copy_src(images: &mut Assets<Image>, render_target: &Handle<Image>) {
    if let Some(image) = images.get_mut(render_target) {
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    }
}
