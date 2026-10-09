use crate::naming::{NameAllocator, comment_text, sanitize};

#[test]
fn given_names_with_unsafe_characters_when_sanitized_then_only_safe_characters_remain() {
    assert_eq!(sanitize("Pillow Block"), "Pillow_Block");
    assert_eq!(sanitize("P/N-123 (rev A)"), "P_N-123__rev_A_");
    assert_eq!(sanitize("café"), "caf_");
    assert_eq!(sanitize("ok.name-1"), "ok.name-1");
    assert_eq!(sanitize(""), "");
}

#[test]
fn given_comment_text_with_dashes_when_sanitized_then_no_dash_dash_remains() {
    assert_eq!(comment_text("a--b"), "a-b");
    assert_eq!(comment_text("line1\nline2"), "line1 line2");
    assert_eq!(comment_text("plain"), "plain");
}

#[test]
fn given_an_empty_raw_name_when_allocated_then_the_fallback_is_used() {
    let mut allocator = NameAllocator::new();
    assert_eq!(allocator.allocate("", "part7"), "part7");
}

#[test]
fn given_repeated_names_when_allocated_then_suffixes_dedupe() {
    let mut allocator = NameAllocator::new();
    assert_eq!(allocator.allocate("box", "part0"), "box");
    assert_eq!(allocator.allocate("box", "part1"), "box_2");
    assert_eq!(allocator.allocate("box", "part2"), "box_3");
}

#[test]
fn given_a_name_that_already_exists_as_a_suffix_when_allocated_then_a_free_suffix_is_found() {
    let mut allocator = NameAllocator::new();
    assert_eq!(allocator.allocate("box_2", "part0"), "box_2");
    assert_eq!(allocator.allocate("box", "part1"), "box");
    assert_eq!(allocator.allocate("box", "part2"), "box_3");
}
