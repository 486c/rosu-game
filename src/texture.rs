use std::{io::Cursor, path::Path};
use image::{imageops::FilterType, io::Reader as ImageReader, DynamicImage, GenericImageView, RgbaImage};
use wgpu::{ShaderStages, BindingType, TextureSampleType, TextureViewDimension};

use crate::graphics::Graphics;

pub struct AtlasTexture {
    texture: Texture,
    /// Number of images inside atlas
    images: u32,

    /// Width of single image in atlas
    image_width: f32,

    /// Height of single image in atlas
    image_height: f32,
}

/// Atlas Texture of multiple images
/// Currently it is vertical only for simplicity
impl AtlasTexture {
    pub fn from_images(graphics: &Graphics, images: &[DynamicImage]) -> Self {
        // Provided images should be the same size in order to fit propely
        for i in images {
            for j in images {
                assert_eq!(i.dimensions(), j.dimensions());
            }
        }

        let (f_w, f_h) = images.get(0).unwrap().dimensions();
        
        // Placing it in one row
        let total_height = f_h;
        let total_width = f_w * images.len() as u32;

        let mut atlas_rgba_image = RgbaImage::new(total_width, total_height);

        let mut current_x = 0;
        for img in images {
            for y in 0..f_h {
                for x in 0..f_w {
                    let pixel = img.get_pixel(x, y);
                    atlas_rgba_image.put_pixel(current_x + x, y, pixel);
                }
            }
            current_x += f_w;
        }

        let atlas_image = DynamicImage::ImageRgba8(atlas_rgba_image);

        let atlas_texture = Texture::from_image(
            atlas_image, graphics
        );

        Self {
            texture: atlas_texture,
            images: images.len() as u32,
            image_width: f_w as f32,
            image_height: f_h as f32,
        }
    }

    pub fn coords_from_index(&self, index: u32) {
        assert!(index > self.images);
    }
    
    #[inline]
    pub fn width(&self) -> f32 {
        self.images as f32 * self.image_width
    }

    #[inline]
    pub fn height(&self) -> f32 {
        self.image_height
    }

    #[inline]
    pub fn image_width(&self) -> f32 {
        self.image_height
    }

    #[inline]
    pub fn image_height(&self) -> f32 {
        self.image_height
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.texture.bind_group
    }
}

pub struct DepthTexture {
    pub view: wgpu::TextureView,
}

impl DepthTexture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn new(graphics: &Graphics, width: u32, height: u32, sample_count: u32) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,

        };
        let desc = wgpu::TextureDescriptor {
            label: Some("another depth texture"),
            size,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = graphics.device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let _sampler = graphics.device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual),
                lod_min_clamp: 0.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            }
        );

        Self { view }
    }
}

pub struct Texture {
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub width: f32,
    pub height: f32,
}

impl Texture {
    pub fn from_path<P: AsRef<Path>>(
        path: P, graphics: &Graphics
    ) -> Self {
        let image = ImageReader::open(path).unwrap()
            .decode().unwrap();

        let width = image.width();
        let height = image.height();

        let image = image.resize(width*2, height*2, FilterType::Lanczos3);

        Self::from_image(image, graphics)
    }

    pub fn from_bytes(bytes: &[u8], graphics: &Graphics) -> Self {
        let image = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format().unwrap()
            .decode().unwrap();

        let width = image.width();
        let height = image.height();

        let image = image.resize(width*2, height*2, FilterType::Lanczos3);

        Self::from_image(image, graphics)
    }

    pub fn default_bind_group_layout(graphics: &Graphics, sample_count: u32) -> wgpu::BindGroupLayout {
        graphics.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_something"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float {
                                filterable: if sample_count > 1 { false } else { true } 
                            },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: if sample_count > 1 { true } else { false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler (
                            wgpu::SamplerBindingType::Filtering
                        ),
                        count: None,
                    },
                ],
            })
    }

    pub fn from_texture(texture: wgpu::Texture, graphics: &Graphics, width: u32, height: u32, sample_count: u32) -> Self {
        let view = texture.create_view(
            &wgpu::TextureViewDescriptor::default()
        );

        let sampler = graphics.device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );
        

        let bind_group_layout = Self::default_bind_group_layout(graphics, sample_count);

        let bind_group = graphics.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &view
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(
                            &sampler
                        ),
                    },
                ],
                label: Some("from_texture_bind"),
            }
        );

        Self {
            width: width as f32,
            height: height as f32,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn from_image(image: DynamicImage, graphics: &Graphics) -> Self {
        let dimensions = image.dimensions();
        let buffer = image.to_rgba8();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = graphics.device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("Whatever"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }
            );

        graphics.queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &buffer,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
            );

        let view = texture.create_view(
            &wgpu::TextureViewDescriptor::default()
        );

        let sampler = graphics.device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );

        let bind_group_layout = graphics.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_something"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float {
                                filterable: true
                            },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler (
                            wgpu::SamplerBindingType::Filtering
                        ),
                        count: None,
                    },
                ],
            });


        let bind_group = graphics.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &view
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(
                            &sampler
                        ),
                    },
                ],
                label: Some("hit_circle_bind"),
            }
        );

        Self {
            //texture,
            //view,
            //sampler,
            width: dimensions.0 as f32,
            height: dimensions.1 as f32,
            bind_group_layout,
            bind_group,
        }
    }
}
