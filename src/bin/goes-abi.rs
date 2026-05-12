use _goes_abi::{
    GoesNativeSequenceRequest, GoesSatelliteBatchRequest, GoesWebTilesRequest, PngCompressionMode,
    ViirsFireRequest, capabilities_json, run_goes_native_sequence, run_goes_satellite_batch,
    run_goes_web_tiles, run_viirs_fire_detection, web_tiles::GoesWebTileLayerMode,
};
use chrono::{DateTime, NaiveDate, Utc};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "goes-abi",
    about = "Standalone GOES ABI native renderer and XYZ tile generator"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Print supported satellites, sectors, products, and outputs.
    Capabilities,
    /// Discover, cache, and render latest GOES ABI products as native fixed-grid PNGs.
    Render(RenderArgs),
    /// Render native fixed-grid crops or sequences for an area/time window.
    NativeSequence(NativeSequenceArgs),
    /// Render transparent XYZ Web Mercator tiles from local GOES ABI C01/C02/C03 files.
    WebTiles(WebTilesArgs),
    /// Fetch or parse VIIRS active-fire detections through NASA FIRMS CSV.
    ViirsFires(ViirsFiresArgs),
}

#[derive(Debug, Args)]
struct RenderArgs {
    #[arg(long, default_value = "goes19")]
    satellite: String,
    #[arg(long, default_value = "ABI-L2-CMIPC")]
    abi_product: String,
    #[arg(long, help = "ABI sector shortcut: conus, full_disk, meso1, or meso2")]
    sector: Option<String>,
    #[arg(long, default_value = "goes_native")]
    domain: String,
    #[arg(long, default_value = "GOES Native")]
    label: String,
    #[arg(long, default_value_t = -127.0)]
    west: f64,
    #[arg(long, default_value_t = -111.0)]
    east: f64,
    #[arg(long, default_value_t = 30.0)]
    south: f64,
    #[arg(long, default_value_t = 44.5)]
    north: f64,
    #[arg(long)]
    out_dir: PathBuf,
    #[arg(long)]
    cache_dir: Option<PathBuf>,
    #[arg(long, value_delimiter = ',')]
    products: Vec<String>,
    #[arg(long, default_value_t = 1400)]
    width: u32,
    #[arg(long, default_value_t = 1100)]
    height: u32,
    #[arg(long, default_value_t = 6)]
    scan_lookback_hours: u32,
    #[arg(long, default_value_t = 2)]
    discovery_retries: u32,
    #[arg(long, default_value_t = 20_000)]
    retry_sleep_ms: u64,
    #[arg(long)]
    no_cache: bool,
    #[arg(long, value_enum, default_value_t = PngCompressionArg::Fast)]
    png_compression: PngCompressionArg,
    #[arg(long)]
    skip_scan_id: Option<String>,
    #[arg(long)]
    allow_high_resolution_full_disk: bool,
    #[arg(long, default_value_t = 1)]
    sequence_count: usize,
    #[arg(long)]
    sequence_gif: bool,
    #[arg(long, default_value_t = 180)]
    sequence_gif_delay_ms: u32,
}

#[derive(Debug, Args)]
struct NativeSequenceArgs {
    #[arg(long, default_value = "goes19")]
    satellite: String,
    #[arg(long, default_value = "ABI-L2-CMIPC")]
    abi_product: String,
    #[arg(long)]
    sector: Option<String>,
    #[arg(long, default_value = "geocolor")]
    product: String,
    #[arg(long, default_value = "native_crop")]
    domain: String,
    #[arg(long, default_value = "Native Crop")]
    label: String,
    #[arg(long, allow_hyphen_values = true)]
    west: f64,
    #[arg(long, allow_hyphen_values = true)]
    east: f64,
    #[arg(long, allow_hyphen_values = true)]
    south: f64,
    #[arg(long, allow_hyphen_values = true)]
    north: f64,
    #[arg(long)]
    out_dir: PathBuf,
    #[arg(long)]
    cache_dir: Option<PathBuf>,
    #[arg(long)]
    start: Option<String>,
    #[arg(long)]
    end: Option<String>,
    #[arg(long, default_value_t = 1)]
    latest_count: usize,
    #[arg(long, default_value_t = 6)]
    scan_lookback_hours: u32,
    #[arg(long)]
    min_step_minutes: Option<u32>,
    #[arg(long)]
    no_cache: bool,
    #[arg(long, default_value_t = 1.0)]
    downsample: f64,
    #[arg(long)]
    max_width: Option<u32>,
    #[arg(long)]
    max_height: Option<u32>,
    #[arg(long, default_value_t = 8)]
    download_workers: usize,
    #[arg(long, default_value_t = 0)]
    render_workers: usize,
    #[arg(long, default_value_t = 1)]
    discovery_retries: u32,
    #[arg(long, default_value_t = 10_000)]
    retry_sleep_ms: u64,
    #[arg(long, value_enum, default_value_t = PngCompressionArg::Fast)]
    png_compression: PngCompressionArg,
}

#[derive(Debug, Args)]
struct WebTilesArgs {
    #[arg(long)]
    channel1: PathBuf,
    #[arg(long)]
    channel2: PathBuf,
    #[arg(long)]
    channel3: PathBuf,
    #[arg(long)]
    channel13: Option<PathBuf>,
    #[arg(long)]
    out_dir: PathBuf,
    #[arg(long, default_value = "goes_geocolor_webmercator")]
    name: String,
    #[arg(long, default_value_t = -165.0)]
    west: f64,
    #[arg(long, default_value_t = -5.0)]
    east: f64,
    #[arg(long, default_value_t = -70.0)]
    south: f64,
    #[arg(long, default_value_t = 70.0)]
    north: f64,
    #[arg(long, default_value_t = 2)]
    min_zoom: u8,
    #[arg(long, default_value_t = 5)]
    max_zoom: u8,
    #[arg(long, default_value_t = 256)]
    tile_size: u32,
    #[arg(long, default_value_t = 0.82)]
    opacity: f64,
    #[arg(long)]
    opaque_clouds: bool,
    #[arg(long, value_enum, default_value_t = WebLayerArg::Geocolor)]
    layer: WebLayerArg,
    #[arg(long)]
    base_url: Option<String>,
    #[arg(long, value_enum, default_value_t = PngCompressionArg::Fast)]
    png_compression: PngCompressionArg,
}

#[derive(Debug, Args)]
struct ViirsFiresArgs {
    #[arg(long, default_value = "VIIRS_NOAA20_NRT")]
    source: String,
    #[arg(
        long,
        help = "NASA FIRMS map key; defaults to FIRMS_MAP_KEY/NASA_FIRMS_MAP_KEY/GOES_ABI_FIRMS_MAP_KEY"
    )]
    map_key: Option<String>,
    #[arg(
        long,
        help = "Parse an existing FIRMS CSV instead of fetching from NASA FIRMS"
    )]
    csv_path: Option<PathBuf>,
    #[arg(long, default_value_t = -127.0)]
    west: f64,
    #[arg(long, default_value_t = -111.0)]
    east: f64,
    #[arg(long, default_value_t = 30.0)]
    south: f64,
    #[arg(long, default_value_t = 44.5)]
    north: f64,
    #[arg(long, help = "Request FIRMS world area instead of the bounding box")]
    world: bool,
    #[arg(long, default_value_t = 1)]
    day_range: u8,
    #[arg(long, help = "Optional FIRMS start date in YYYY-MM-DD")]
    date: Option<String>,
    #[arg(long)]
    out_dir: Option<PathBuf>,
    #[arg(long)]
    no_geojson: bool,
    #[arg(long)]
    min_confidence: Option<String>,
    #[arg(long)]
    min_frp: Option<f64>,
    #[arg(long)]
    limit: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum PngCompressionArg {
    Default,
    Fast,
    Fastest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum WebLayerArg {
    Geocolor,
    Clouds,
}

impl From<PngCompressionArg> for PngCompressionMode {
    fn from(value: PngCompressionArg) -> Self {
        match value {
            PngCompressionArg::Default => Self::Default,
            PngCompressionArg::Fast => Self::Fast,
            PngCompressionArg::Fastest => Self::Fastest,
        }
    }
}

impl From<WebLayerArg> for GoesWebTileLayerMode {
    fn from(value: WebLayerArg) -> Self {
        match value {
            WebLayerArg::Geocolor => Self::Geocolor,
            WebLayerArg::Clouds => Self::Clouds,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::CryptoProvider::install_default(rustls_rustcrypto::provider())
        .map_err(|_| "failed to install rustls crypto provider")?;
    let cli = Cli::parse();
    match cli.command {
        Commands::Capabilities => println!("{}", capabilities_json()),
        Commands::Render(args) => {
            let request = GoesSatelliteBatchRequest {
                satellite: args.satellite,
                abi_product: args.abi_product,
                abi_sector: args.sector,
                domain_slug: args.domain,
                domain_label: args.label,
                bounds: (args.west, args.east, args.south, args.north),
                out_dir: args.out_dir,
                cache_dir: args.cache_dir.unwrap_or_else(_goes_abi::default_cache_dir),
                products: args.products,
                width: args.width,
                height: args.height,
                scan_lookback_hours: args.scan_lookback_hours,
                discovery_retries: args.discovery_retries,
                retry_sleep_ms: args.retry_sleep_ms,
                use_cache: !args.no_cache,
                download_glm: false,
                glm_fetch_count: 0,
                glm_lookback_hours: 0,
                glm_max_age_min: 0.0,
                png_compression: args.png_compression.into(),
                skip_scan_id: args.skip_scan_id,
                auto_bounds: true,
                allow_high_resolution_full_disk: args.allow_high_resolution_full_disk,
                sequence_count: args.sequence_count,
                sequence_gif: args.sequence_gif,
                sequence_gif_delay_ms: args.sequence_gif_delay_ms,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&run_goes_satellite_batch(&request)?)?
            );
        }
        Commands::NativeSequence(args) => {
            let request = GoesNativeSequenceRequest {
                satellite: args.satellite,
                abi_product: args.abi_product,
                abi_sector: args.sector,
                product: args.product,
                domain_slug: args.domain,
                domain_label: args.label,
                bounds: (args.west, args.east, args.south, args.north),
                out_dir: args.out_dir,
                cache_dir: args.cache_dir.unwrap_or_else(_goes_abi::default_cache_dir),
                start_time_utc: parse_optional_time(args.start.as_deref())?,
                end_time_utc: parse_optional_time(args.end.as_deref())?,
                latest_count: args.latest_count,
                scan_lookback_hours: args.scan_lookback_hours,
                min_step_minutes: args.min_step_minutes,
                use_cache: !args.no_cache,
                downsample: args.downsample,
                max_width: args.max_width,
                max_height: args.max_height,
                download_workers: args.download_workers,
                render_workers: args.render_workers,
                discovery_retries: args.discovery_retries,
                retry_sleep_ms: args.retry_sleep_ms,
                png_compression: args.png_compression.into(),
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&run_goes_native_sequence(&request)?)?
            );
        }
        Commands::WebTiles(args) => {
            let request = GoesWebTilesRequest {
                channel1: args.channel1,
                channel2: args.channel2,
                channel3: args.channel3,
                channel13: args.channel13,
                out_dir: args.out_dir,
                name: args.name,
                bounds: (args.west, args.east, args.south, args.north),
                min_zoom: args.min_zoom,
                max_zoom: args.max_zoom,
                tile_size: args.tile_size,
                opacity: args.opacity,
                opaque_clouds: args.opaque_clouds,
                layer: args.layer.into(),
                base_url: args.base_url,
                png_compression: args.png_compression.into(),
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&run_goes_web_tiles(&request)?)?
            );
        }
        Commands::ViirsFires(args) => {
            let request = ViirsFireRequest {
                source: args.source,
                map_key: args.map_key,
                csv_path: args.csv_path,
                csv_text: None,
                bounds: (args.west, args.east, args.south, args.north),
                world: args.world,
                day_range: args.day_range,
                date: parse_optional_date(args.date.as_deref())?,
                out_dir: args.out_dir,
                write_geojson: !args.no_geojson,
                min_confidence: args.min_confidence,
                min_frp: args.min_frp,
                limit: args.limit,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&run_viirs_fire_detection(&request)?)?
            );
        }
    }
    Ok(())
}

fn parse_optional_time(
    raw: Option<&str>,
) -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
    raw.map(|value| {
        DateTime::parse_from_rfc3339(value)
            .map(|time| time.with_timezone(&Utc))
            .map_err(|err| err.into())
    })
    .transpose()
}

fn parse_optional_date(raw: Option<&str>) -> Result<Option<NaiveDate>, Box<dyn std::error::Error>> {
    raw.map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|err| err.into()))
        .transpose()
}
