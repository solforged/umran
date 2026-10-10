use serde_json::Value;
use umran_web::Bench;

#[test]
fn forming_contract_matches_map_at_every_size_and_generation() {
    for seed in [0, 7, 21] {
        for size in ["small", "medium", "large", "vast"] {
            let mut bench = Bench::new(seed, size).unwrap();
            let map_json = bench.map().unwrap();
            let map: Value = serde_json::from_str(&map_json).unwrap();
            let regions = map["regions"].as_array().unwrap();
            let json = bench.forming().unwrap();
            let forming: Value = serde_json::from_str(&json).unwrap();
            assert_eq!(forming.as_object().unwrap().len(), 1);
            let frames = forming["frames"].as_array().unwrap();
            assert_eq!(frames.len(), 12);
            for frame in frames {
                assert_eq!(frame.as_object().unwrap().len(), 2);
                let land = frame["land"].as_str().unwrap();
                let plate = frame["plate"].as_str().unwrap();
                assert_eq!(land.len(), regions.len());
                assert_eq!(plate.len(), regions.len());
                assert!(land.bytes().all(|c| c == b'0' || c == b'1'));
                assert!(
                    plate
                        .bytes()
                        .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
                );
            }
            let land: String = regions
                .iter()
                .map(|r| if r["terrain"] == "sea" { '0' } else { '1' })
                .collect();
            assert_eq!(frames[11]["land"], land);
            assert_eq!(frames[11]["plate"], frames[10]["plate"]);
            assert_eq!(bench.map().unwrap(), map_json);
            bench.run_until_event(2);
            assert_eq!(bench.forming().unwrap(), json);
            assert_eq!(bench.read(0, "").unwrap().forming().unwrap(), json);
        }
    }
    println!(
        "forming: 12-frame JSON, all region lengths/plate chars, authoritative coast, immutable map, later generations/readings; 3 seeds x 4 sizes"
    );
}

#[test]
fn forming_rejects_every_older_geography() {
    for geography in [
        "spherical-v1",
        "continental-v2",
        "continental-v3",
        "continental-v4",
        "continental-v5",
    ] {
        let bench = Bench::with_geography(7, "small", geography).unwrap();
        assert_eq!(
            bench.forming().unwrap_err(),
            "Forming frames require continental-v6 geography."
        );
    }
}
