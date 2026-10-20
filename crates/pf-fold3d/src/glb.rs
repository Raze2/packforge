//! Minimal binary glTF 2.0 writer: one mesh, one double-sided material, metres.

use crate::{Folded, Vec3};
use serde_json::json;

const GLB_MAGIC: u32 = 0x4654_6C67; // "glTF"
const CHUNK_JSON: u32 = 0x4E4F_534A; // "JSON"
const CHUNK_BIN: u32 = 0x004E_4942; // "BIN\0"

/// Serialises the folded box as GLB. `rgba` is the board colour (linear 0..1).
pub fn to_glb(folded: &Folded, rgba: [f32; 4]) -> Vec<u8> {
    let mm_to_m = 0.001;
    let mut positions: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];

    for p in &folded.panels {
        let base = (positions.len() / 3) as u32;
        let n = unit(p.normal);
        for c in &p.corners {
            let xyz = [
                (c.x * mm_to_m) as f32,
                (c.y * mm_to_m) as f32,
                (c.z * mm_to_m) as f32,
            ];
            for k in 0..3 {
                min[k] = min[k].min(xyz[k]);
                max[k] = max[k].max(xyz[k]);
            }
            positions.extend_from_slice(&xyz);
            normals.extend_from_slice(&[n.x as f32, n.y as f32, n.z as f32]);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let mut bin: Vec<u8> = Vec::new();
    let pos_offset = bin.len();
    positions
        .iter()
        .for_each(|f| bin.extend_from_slice(&f.to_le_bytes()));
    let nrm_offset = bin.len();
    normals
        .iter()
        .for_each(|f| bin.extend_from_slice(&f.to_le_bytes()));
    let idx_offset = bin.len();
    indices
        .iter()
        .for_each(|i| bin.extend_from_slice(&i.to_le_bytes()));
    let bin_len = bin.len();
    while bin.len() % 4 != 0 {
        bin.push(0);
    }

    let vertex_count = positions.len() / 3;
    let gltf = json!({
        "asset": { "version": "2.0", "generator": "PackForge" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0, "name": "box" }],
        "meshes": [{
            "name": "box",
            "primitives": [{
                "attributes": { "POSITION": 0, "NORMAL": 1 },
                "indices": 2,
                "material": 0
            }]
        }],
        "materials": [{
            "name": "board",
            "doubleSided": true,
            "pbrMetallicRoughness": {
                "baseColorFactor": rgba,
                "metallicFactor": 0.0,
                "roughnessFactor": 0.9
            }
        }],
        "buffers": [{ "byteLength": bin.len() }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": pos_offset, "byteLength": nrm_offset - pos_offset, "target": 34962 },
            { "buffer": 0, "byteOffset": nrm_offset, "byteLength": idx_offset - nrm_offset, "target": 34962 },
            { "buffer": 0, "byteOffset": idx_offset, "byteLength": bin_len - idx_offset, "target": 34963 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": vertex_count, "type": "VEC3",
              "min": if vertex_count > 0 { min.to_vec() } else { vec![0.0; 3] },
              "max": if vertex_count > 0 { max.to_vec() } else { vec![0.0; 3] } },
            { "bufferView": 1, "componentType": 5126, "count": vertex_count, "type": "VEC3" },
            { "bufferView": 2, "componentType": 5125, "count": indices.len(), "type": "SCALAR" }
        ]
    });
    let mut json_bytes = serde_json::to_vec(&gltf).unwrap_or_default();
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }

    let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&GLB_MAGIC.to_le_bytes());
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_JSON.to_le_bytes());
    out.extend_from_slice(&json_bytes);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_BIN.to_le_bytes());
    out.extend_from_slice(&bin);
    out
}

fn unit(v: Vec3) -> Vec3 {
    let l = v.length();
    if l > 0.0 {
        Vec3::new(v.x / l, v.y / l, v.z / l)
    } else {
        Vec3::new(0.0, 0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fold, Panel3};

    fn read_u32(b: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
    }

    #[test]
    fn glb_structure_is_valid() {
        let folded = Folded {
            progress: 1.0,
            root: "a".into(),
            panels: vec![Panel3 {
                id: "a".into(),
                corners: [
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(100.0, 0.0, 0.0),
                    Vec3::new(100.0, 50.0, 0.0),
                    Vec3::new(0.0, 50.0, 0.0),
                ],
                normal: Vec3::new(0.0, 0.0, 1.0),
                depth: 0,
            }],
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(100.0, 50.0, 0.0),
        };
        let glb = to_glb(&folded, [0.8, 0.65, 0.45, 1.0]);
        assert_eq!(read_u32(&glb, 0), GLB_MAGIC);
        assert_eq!(read_u32(&glb, 4), 2);
        assert_eq!(read_u32(&glb, 8) as usize, glb.len());
        let json_len = read_u32(&glb, 12) as usize;
        assert_eq!(read_u32(&glb, 16), CHUNK_JSON);
        assert_eq!(json_len % 4, 0);
        let doc: serde_json::Value =
            serde_json::from_slice(&glb[20..20 + json_len]).expect("json chunk");
        let bin_at = 20 + json_len;
        let bin_len = read_u32(&glb, bin_at) as usize;
        assert_eq!(read_u32(&glb, bin_at + 4), CHUNK_BIN);
        assert_eq!(
            bin_len,
            doc["buffers"][0]["byteLength"].as_u64().expect("len") as usize
        );
        // Every buffer view fits inside the binary chunk; accessor counts match.
        for bv in doc["bufferViews"].as_array().expect("views") {
            let end = bv["byteOffset"].as_u64().expect("o") + bv["byteLength"].as_u64().expect("l");
            assert!(end as usize <= bin_len);
        }
        assert_eq!(doc["accessors"][0]["count"], 4);
        assert_eq!(doc["accessors"][2]["count"], 6);
        let max_x = doc["accessors"][0]["max"][0].as_f64().expect("max");
        assert!((max_x - 0.1).abs() < 1e-6, "{max_x}"); // 100 mm in metres (f32)
    }

    #[test]
    fn glb_is_deterministic() {
        let model = pf_model::FoldModel {
            panels: vec![pf_model::Panel {
                id: "p".into(),
                origin: pf_geom::Point::new(0.0, 0.0),
                width: 10.0,
                height: 10.0,
            }],
            hinges: vec![],
            thickness: 1.0,
        };
        let f = fold(&model, 1.0).expect("fold");
        assert_eq!(to_glb(&f, [1.0; 4]), to_glb(&f, [1.0; 4]));
    }
}
