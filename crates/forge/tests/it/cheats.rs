use std::path::PathBuf;

forgetest!(fdk, |_prj, cmd| {
    let testdata =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata").canonicalize().unwrap();
    cmd.current_dir(&testdata);
    cmd.forge_fuse().args(["test", "--match-path", "default/cheats/Fdk.t.sol"]).assert_success();
});
