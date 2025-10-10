/// User-facing strings for jail-ai
///
/// This module contains all user-facing strings as constants for future i18n support.
/// Currently only English strings are provided, but this structure allows for easy
/// addition of other languages in the future.
/// Update and upgrade related messages
pub const UPDATE_AVAILABLE: &str = "\n🔄 Update available for your jail environment!";
pub const OUTDATED_LAYERS_DETECTED: &str = "\n📦 Outdated layers detected:";
pub const OUTDATED_LAYERS_EXPLAIN: &str = "\nThis typically happens after upgrading the jail-ai binary.\nLayers contain updated tools, dependencies, or security patches.";
pub const CONTAINER_IMAGE_MISMATCH: &str = "\n🐳 Container image mismatch:";
pub const CURRENT: &str = "  Current:  {}";
pub const EXPECTED: &str = "  Expected: {}";
pub const RECOMMENDATION_USE_UPGRADE: &str = "\n💡 Recommendation: Use --upgrade to:";
pub const REBUILD_OUTDATED_LAYERS: &str = "  • Rebuild outdated layers with latest definitions";
pub const RECREATE_CONTAINER: &str = "  • Recreate container with the correct image";
pub const ENSURE_LATEST_TOOLS: &str = "  • Ensure you have the latest tools and security patches";
pub const DATA_PRESERVED: &str = "\nYour data in /home/agent will be preserved during the rebuild.";
pub const WOULD_YOU_LIKE_REBUILD: &str = "\nWould you like to rebuild now? (y/N): ";

/// Status and progress messages
pub const CHECKING_UPDATES: &str = "Checking for updates...";
pub const USER_CHOSE_UPGRADE: &str = "User chose to upgrade";
pub const USER_DECLINED_UPGRADE: &str = "User declined rebuild, continuing with existing container";
pub const CONTAINER_UP_TO_DATE: &str = "Container and layers are up to date";

/// Jail creation messages
pub const CREATING_NEW_JAIL: &str = "Creating new jail: {}";
pub const RECREATING_JAIL_UPGRADE: &str = "Recreating jail '{}' due to --upgrade or --layers";
pub const RECREATING_JAIL_DETECTED_UPDATES: &str = "Recreating jail '{}' due to detected updates";

/// Helper function for single argument formatting
pub fn format_string(template: &str, arg: &dyn std::fmt::Display) -> String {
    template.replace("{}", &arg.to_string())
}

