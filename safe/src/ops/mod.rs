pub(crate) fn is_manifest_supported_operation(nickname: &str) -> bool {
    matches!(
        nickname,
        "avg"
            | "black"
            | "copy"
            | "crop"
            | "pngload"
            | "pngload_buffer"
            | "pngsave"
            | "pngsave_buffer"
    )
}
