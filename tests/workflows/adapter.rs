use aimt::plugins::adapter::{HostAction, Suggestion, WorkflowAdapter};
use aimt::plugins::opencode::OpenCodeAdapter;
#[test]
fn opencode_translates_search_to_tool_call() {
    let adapter = OpenCodeAdapter;
    assert_eq!(adapter.id(), "opencode");
    assert!(matches!(
        adapter.translate(Suggestion::Execute(
            aimt::workflows::capabilities::Capability::Search
        )),
        HostAction::ToolCall(_)
    ));
}
