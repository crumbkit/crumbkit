//! Tests for the records that a tool declares.

use crate::help::{Help, Leaf};
use crate::pick::Menu;
use crate::tool::{Exit, Failures, Output};
use crate::{JsonLayout, Tool};

#[test]
fn a_tool_that_declares_nothing_has_crumbkit_s_defaults_in_every_record() {
    const TOOL: Tool = Tool::new();
    assert_eq!(TOOL.get_help().get_leaves().len(), 0);
    assert_eq!(TOOL.get_failures().get_exits().len(), 0);
    assert_eq!(TOOL.get_output().get_budget(), None);
    assert_eq!(TOOL.get_output().get_json_layout(), JsonLayout::OneLine);
    assert_eq!(TOOL.get_output().get_without_terminal_detection(), None);
    assert!(TOOL.get_picker().is_none());
}

#[test]
fn a_tool_that_sets_one_record_leaves_the_other_records_at_their_defaults() {
    const LEAVES: &[Leaf] = &[Leaf::new(&["books"])];
    const TOOL: Tool = Tool::new().help(Help::new().leaves(LEAVES));
    assert_eq!(TOOL.get_help().get_leaves().len(), 1);
    assert_eq!(TOOL.get_output().get_budget(), None);
    assert_eq!(TOOL.get_failures().get_exits().len(), 0);
}

#[test]
fn each_record_is_built_in_a_const_and_read_back_through_its_getters() {
    const TOOL: Tool = Tool::new()
        .failures(Failures::new().exits(&[Exit::new(3, "The shelf was locked.")]))
        .output(Output::new().budget(64, "A small pipe."));
    assert_eq!(TOOL.get_output().get_budget(), Some((64, "A small pipe.")));
    assert_eq!(TOOL.get_failures().get_exits().len(), 1);
}

#[test]
fn a_tool_with_a_picker_is_a_static_that_threads_share() {
    fn shared<T: Send + Sync>(value: &T) -> &T {
        value
    }

    static MENU: Menu = Menu::new();
    static TOOL: Tool = Tool::new().picker(&MENU);
    let tool = shared(&TOOL);
    let picked = std::thread::scope(|scope| {
        scope
            .spawn(|| tool.get_picker().is_some())
            .join()
            .expect("the thread reads the picker")
    });
    assert!(picked);
}
