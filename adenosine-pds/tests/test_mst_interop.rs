use adenosine_pds::RepoStore;
use libipld::Cid;
use std::collections::BTreeMap;
use std::str::FromStr;

#[test]
fn test_known_maps() {
    let mut repo = RepoStore::open_ephemeral().unwrap();
    let cid1 =
        Cid::from_str("bafyreie5cvv4h45feadgeuwhbcutmh6t2ceseocckahdoe6uat64zmz454").unwrap();

    let empty_map: BTreeMap<String, Cid> = Default::default();
    assert_eq!(
        repo.mst_from_map(&empty_map).unwrap().to_string(),
        "bafyreie5737gdxlw5i64vzichcalba3z2v5n6icifvx5xytvske7mr3hpm"
    );

    let mut trivial_map: BTreeMap<String, Cid> = Default::default();
    trivial_map.insert("asdf".to_string(), cid1.clone());
    assert_eq!(
        repo.mst_from_map(&trivial_map).unwrap().to_string(),
        "bafyreidaftbr35xhh4lzmv5jcoeufqjh75ohzmz6u56v7n2ippbtxdgqqe"
    );

    let mut singlelayer2_map: BTreeMap<String, Cid> = Default::default();
    singlelayer2_map.insert("com.example.record/9ba1c7247ede".to_string(), cid1.clone());
    assert_eq!(
        repo.mst_from_map(&singlelayer2_map).unwrap().to_string(),
        "bafyreid4g5smj6ukhrjasebt6myj7wmtm2eijouteoyueoqgoh6vm5jkae"
    );

    let mut simple_map: BTreeMap<String, Cid> = Default::default();
    simple_map.insert("asdf".to_string(), cid1.clone());
    simple_map.insert("88bfafc7".to_string(), cid1.clone());
    simple_map.insert("2a92d355".to_string(), cid1.clone());
    simple_map.insert("app.bsky.feed.post/454397e440ec".to_string(), cid1.clone());
    simple_map.insert("app.bsky.feed.post/9adeb165882c".to_string(), cid1.clone());
    assert_eq!(
        repo.mst_from_map(&simple_map).unwrap().to_string(),
        "bafyreiecb33zh7r2sc3k2wthm6exwzfktof63kmajeildktqc25xj6qzx4"
    );
}

// TODO: behavior of these wide-char keys is undefined behavior in string MST
#[ignore]
#[test]
fn test_tricky_map() {
    let mut repo = RepoStore::open_ephemeral().unwrap();
    let cid1 =
        Cid::from_str("bafyreie5cvv4h45feadgeuwhbcutmh6t2ceseocckahdoe6uat64zmz454").unwrap();

    let mut tricky_map: BTreeMap<String, Cid> = Default::default();
    tricky_map.insert("".to_string(), cid1.clone());
    tricky_map.insert("jalapeño".to_string(), cid1.clone());
    tricky_map.insert("coöperative".to_string(), cid1.clone());
    tricky_map.insert("coüperative".to_string(), cid1.clone());
    tricky_map.insert("abc\x00".to_string(), cid1.clone());
    assert_eq!(
        repo.mst_from_map(&tricky_map).unwrap().to_string(),
        "bafyreiecb33zh7r2sc3k2wthm6exwzfktof63kmajeildktqc25xj6qzx4"
    );
}

#[test]
fn test_trims_top() {
    // "trims top of tree on delete"

    use adenosine_pds::mst::print_mst_keys;
    let mut repo = RepoStore::open_ephemeral().unwrap();
    let cid1 =
        Cid::from_str("bafyreie5cvv4h45feadgeuwhbcutmh6t2ceseocckahdoe6uat64zmz454").unwrap();
    let l1root = "bafyreihuyj2vzb2vjw3yhxg6dy25achg5fmre6gg5m6fjtxn64bqju4dee";
    let l0root = "bafyreibmijjc63mekkjzl3v2pegngwke5u6cu66g75z6uw27v64bc6ahqi";

    // NOTE: this test doesn't do much in this case of rust implementation
    let mut trim_map: BTreeMap<String, Cid> = Default::default();
    trim_map.insert("com.example.record/40c73105b48f".to_string(), cid1.clone()); // level 0
    trim_map.insert("com.example.record/e99bf3ced34b".to_string(), cid1.clone()); // level 0
    trim_map.insert("com.example.record/893e6c08b450".to_string(), cid1.clone()); // level 0
    trim_map.insert("com.example.record/9cd8b6c0cc02".to_string(), cid1.clone()); // level 0
    trim_map.insert("com.example.record/cbe72d33d12a".to_string(), cid1.clone()); // level 0
    trim_map.insert("com.example.record/a15e33ba0f6c".to_string(), cid1.clone()); // level 1
    let trim_before_cid = repo.mst_from_map(&trim_map).unwrap();
    print_mst_keys(&mut repo.db, &trim_before_cid).unwrap();
    assert_eq!(trim_before_cid.to_string(), l1root);

    // NOTE: if we did mutations in-place, this is where we would mutate

    trim_map.remove("com.example.record/a15e33ba0f6c");
    let trim_after_cid = repo.mst_from_map(&trim_map).unwrap();
    assert_eq!(trim_after_cid.to_string(), l0root);
}

#[test]
fn test_insertion() {
    // "handles insertion that splits two layers down"

    let mut repo = RepoStore::open_ephemeral().unwrap();
    let cid1 =
        Cid::from_str("bafyreie5cvv4h45feadgeuwhbcutmh6t2ceseocckahdoe6uat64zmz454").unwrap();
    let l1root = "bafyreiagt55jzvkenoa4yik77dhomagq2uj26ix4cijj7kd2py2u3s43ve";
    let l2root = "bafyreiddrz7qbvfattp5dzzh4ldohsaobatsg7f5l6awxnmuydewq66qoa";

    // TODO: actual mutation instead of rebuild from scratch
    let mut insertion_map: BTreeMap<String, Cid> = Default::default();
    insertion_map.insert("com.example.record/403e2aeebfdb".to_string(), cid1.clone()); // A; level 0
    insertion_map.insert("com.example.record/40c73105b48f".to_string(), cid1.clone()); // B; level 0
    insertion_map.insert("com.example.record/645787eb4316".to_string(), cid1.clone()); // C; level 0
    insertion_map.insert("com.example.record/7ca4e61d6fbc".to_string(), cid1.clone()); // D; level 1
    insertion_map.insert("com.example.record/893e6c08b450".to_string(), cid1.clone()); // E; level 0
    insertion_map.insert("com.example.record/9cd8b6c0cc02".to_string(), cid1.clone()); // G; level 0
    insertion_map.insert("com.example.record/cbe72d33d12a".to_string(), cid1.clone()); // H; level 0
    insertion_map.insert("com.example.record/dbea731be795".to_string(), cid1.clone()); // I; level 1
    insertion_map.insert("com.example.record/e2ef555433f2".to_string(), cid1.clone()); // J; level 0
    insertion_map.insert("com.example.record/e99bf3ced34b".to_string(), cid1.clone()); // K; level 0
    insertion_map.insert("com.example.record/f728ba61e4b6".to_string(), cid1.clone()); // L; level 0
    let insertion_before_cid = repo.mst_from_map(&insertion_map).unwrap();
    assert_eq!(insertion_before_cid.to_string(), l1root);

    insertion_map.insert("com.example.record/9ba1c7247ede".to_string(), cid1.clone());
    let insertion_after_cid = repo.mst_from_map(&insertion_map).unwrap();
    assert_eq!(insertion_after_cid.to_string(), l2root);
}

#[test]
fn test_higher_layers() {
    // "handles new layers that are two higher than existing"

    use adenosine_pds::mst::print_mst_keys;
    let mut repo = RepoStore::open_ephemeral().unwrap();
    let cid1 =
        Cid::from_str("bafyreie5cvv4h45feadgeuwhbcutmh6t2ceseocckahdoe6uat64zmz454").unwrap();
    let l0root = "bafyreicivoa3p3ttcebdn2zfkdzenkd2uk3gxxlaz43qvueeip6yysvq2m";
    let l2root = "bafyreidwoqm6xlewxzhrx6ytbyhsazctlv72txtmnd4au6t53z2vpzn7wa";
    let l2root2 = "bafyreiapru27ce4wdlylk5revtr3hewmxhmt3ek5f2ypioiivmdbv5igrm";

    // TODO: actual mutation instead of rebuild from scratch
    let mut higher_map: BTreeMap<String, Cid> = Default::default();
    higher_map.insert("com.example.record/403e2aeebfdb".to_string(), cid1.clone()); // A; level 0
    higher_map.insert("com.example.record/cbe72d33d12a".to_string(), cid1.clone()); // C; level 0
    let higher_before_cid = repo.mst_from_map(&higher_map).unwrap();
    assert_eq!(higher_before_cid.to_string(), l0root);

    higher_map.insert("com.example.record/9ba1c7247ede".to_string(), cid1.clone()); // B; level 2
    let higher_after_cid = repo.mst_from_map(&higher_map).unwrap();
    print_mst_keys(&mut repo.db, &higher_after_cid).unwrap();
    assert_eq!(higher_after_cid.to_string(), l2root);

    higher_map.insert("com.example.record/fae7a851fbeb".to_string(), cid1.clone()); // D; level 1
    let higher_after_cid = repo.mst_from_map(&higher_map).unwrap();
    assert_eq!(higher_after_cid.to_string(), l2root2);
}
