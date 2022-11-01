
use adenosine_pds::repro_mst;
use std::path::PathBuf;
use std::str::FromStr;

#[test]
fn test_repro_mst() {
    repro_mst(&PathBuf::from_str("./tests/example_repo.car").unwrap()).unwrap();
    repro_mst(&PathBuf::from_str("./tests/bigger.car").unwrap()).unwrap();
}
