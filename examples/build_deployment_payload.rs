//! Build a `DeploymentDetails` payload without making any network calls.
//!
//! Demonstrates the fluent builder pattern for the DeployNaut deployment
//! payload. Useful for testing what JSON the binary will send without
//! needing a live DeployNaut instance.
//!
//! Run with: `cargo run --example build_deployment_payload`

use nautpie::deployment_details::DeploymentDetails;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use the consuming builder. Each `with_*` call returns a modified copy.
    let details = DeploymentDetails::new()
        .ref_("abc123def456abc123def456abc123def4567890")
        .ref_type("sha")
        .title("[CD] v1.2.3")
        .summary("Branch:main")
        .bypass_and_start(true)
        .locked(false);

    // `values()` renders to a `serde_json::Value` ready to send. Empty and
    // null fields are stripped (matches PHP's `array_filter($details,
    // 'isNotNull')`).
    let json = details.values();
    let pretty = serde_json::to_string_pretty(&json)?;
    println!("{pretty}");

    // Sanity-check the shape. The `ref` field should be present, the default
    // title `[CI] Deployment` should be replaced by `[CD] v1.2.3`, and the
    // `bypass_and_start` boolean should be true.
    assert_eq!(json["ref"], "abc123def456abc123def456abc123def4567890");
    assert_eq!(json["title"], "[CD] v1.2.3");
    assert_eq!(json["bypass_and_start"], true);

    Ok(())
}
