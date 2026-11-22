//! PEN-A01..08. Original fixtures and admitted observable tables only.
use operandi_core::*;
use serde_json::{Value, json};

#[test]
fn integration_literal_v9_polyline_profiles_and_older_version_refusal() {
    // Construct wire fields independently of the v9 encoder using the original
    // I1 source fixture as the surrounding native document.
    let mut doc: Value =
        serde_json::from_slice(include_bytes!("fixtures/i1/cube-source.opscene")).unwrap();
    for closed in [false, true] {
        doc["version"] = json!(9);
        doc["scene"]["items"][0]["mesh"]["attributes"]["representations"] =
            json!([{ "Polyline": { "closed": closed } }, "Face", "Face", "Face", "Face", "Face"]);
        let scene = persistence::decode(&serde_json::to_vec(&doc).unwrap()).unwrap();
        let mesh = &scene.items[0].mesh;
        assert_eq!(mesh.element_spans(0).count(), if closed { 4 } else { 3 });
        assert_eq!(
            mesh.attributes.representations[0],
            PolygonRepresentation::Polyline {
                closed,
                curve: Default::default()
            }
        );
        let encoded: Value = serde_json::from_slice(&persistence::encode(&scene).unwrap()).unwrap();
        assert_eq!(encoded["version"], 9);
        for version in 1..9 {
            doc["version"] = json!(version);
            assert!(persistence::decode(&serde_json::to_vec(&doc).unwrap()).is_err());
        }
    }
}
