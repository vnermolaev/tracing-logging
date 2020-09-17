use std::time::Duration;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use opentelemetry::api::Provider;
use rand::distributions::Distribution;
use rand_distr::num_traits::ToPrimitive;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

struct Dummy;

#[tracing::instrument(skip(_dummy))]
async fn heavy_work(id: String, units: u64, _dummy: Dummy) -> String {
    for i in 1..=units {
        tokio::time::delay_for(Duration::from_secs(1)).await;
        tracing::info!("{} has been working for {} units", id, i);
    }

    id
}

#[tokio::main]
async fn main() {
    init_tracer().expect("Tracer setup failed");
    let root = tracing::span!(tracing::Level::TRACE, "lifecycle");
    let _enter = root.enter();

    // let subscriber = tracing_subscriber::FmtSubscriber::builder()
    //     .with_max_level(tracing::Level::TRACE)
    //     .finish();

    // tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let rng = rand::thread_rng();
    let normal = rand_distr::Normal::new(5.0, 1.0).unwrap();

    let mut finished_work = normal
        .sample_iter(rng)
        .take(10)
        .map(|t| heavy_work(t.to_string(), t.to_u64().expect("Must convert"), Dummy))
        .collect::<FuturesUnordered<_>>();

    while let Some(id) = finished_work.next().await {
        tracing::info!("{} has completed", id);
    }
}

fn init_tracer() -> Result<(), Box<dyn std::error::Error>> {
    let exporter = opentelemetry_jaeger::Exporter::builder()
        .with_agent_endpoint("127.0.0.1:6831".parse().unwrap())
        .with_process(opentelemetry_jaeger::Process {
            service_name: "Test-run".to_string(),
            tags: Vec::new(),
        })
        .init()?;
    let provider = opentelemetry::sdk::Provider::builder()
        .with_simple_exporter(exporter)
        .with_config(opentelemetry::sdk::Config {
            default_sampler: Box::new(opentelemetry::sdk::Sampler::AlwaysOn),
            ..Default::default()
        })
        .build();
    let tracer = provider.get_tracer("tracing");

    let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing_subscriber::registry()
        .with(opentelemetry)
        .try_init()?;

    Ok(())
}
