use integration::TestHarness;

#[test]
fn first() {
    let harness = TestHarness::new("first");

    harness.write_file_content("README.md", "# hello world");
    harness.commit("README.md", "chore: Add README");
    harness.push("main");

    harness.retain();
}
