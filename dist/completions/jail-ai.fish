# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_jail_ai_global_optspecs
	string join \n v/verbose q/quiet h/help
end

function __fish_jail_ai_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_jail_ai_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_jail_ai_using_subcommand
	set -l cmd (__fish_jail_ai_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c jail-ai -n "__fish_jail_ai_needs_command" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_needs_command" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_needs_command" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_needs_command" -f -a "create" -d 'Create a new jail'
complete -c jail-ai -n "__fish_jail_ai_needs_command" -f -a "remove" -d 'Remove a jail'
complete -c jail-ai -n "__fish_jail_ai_needs_command" -f -a "status" -d 'Show jail status'
complete -c jail-ai -n "__fish_jail_ai_needs_command" -f -a "save" -d 'Save jail configuration to file'
complete -c jail-ai -n "__fish_jail_ai_needs_command" -f -a "agents" -d 'Run AI agents. Agent variants are auto-generated from agents/mod.rs'
complete -c jail-ai -n "__fish_jail_ai_needs_command" -f -a "list" -d 'List all jails'
complete -c jail-ai -n "__fish_jail_ai_needs_command" -f -a "clean-all" -d 'Stop and remove all jail-ai containers'
complete -c jail-ai -n "__fish_jail_ai_needs_command" -f -a "upgrade" -d 'Upgrade jail by recreating it with the latest image'
complete -c jail-ai -n "__fish_jail_ai_needs_command" -f -a "completions" -d 'Generate shell completions and print them to stdout'
complete -c jail-ai -n "__fish_jail_ai_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -s b -l backend -d 'Backend type (only \'podman\' is supported, kept for compatibility)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -s i -l image -d 'Base image (e.g., localhost/jail-ai-env:latest, alpine:latest)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -s m -l mount -d 'Bind mount (format: source:target[:ro])' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -s p -l port -d 'Port mapping from host to container (format: host_port:container_port or host_port:container_port/protocol)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -s e -l env -d 'Environment variable (format: KEY=VALUE)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l memory -d 'Memory limit in MB' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l cpu -d 'CPU quota percentage (0-100)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -s c -l config -d 'Load configuration from file' -r -F
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l workspace-path -d 'Custom workspace path inside jail (default: /workspace)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l layers -d 'Force specific layers (comma-separated, e.g., "base,rust,python")' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l no-network -d 'Disable network access'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l host-network -d 'Use host networking (--network=host) instead of private networking Less secure but provides full access to host network services'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l no-workspace -d 'Skip auto-mounting current working directory to /workspace'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l agent-configs -d 'Mount all agent config directories'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l git-gpg -d 'Enable git and GPG configuration mapping'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l upgrade -d 'Upgrade: rebuild outdated layers and recreate container'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l isolated -d 'Use isolated project-specific images (workspace hash tag) instead of shared layer-based images'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l no-nix -d 'Skip nix layer (by default, nix takes precedence over other language layers)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l no-block-host -d 'Disable eBPF-based host blocking (allows connections to host IPs) [default: enabled]'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -l podman -d 'Enable Podman-in-Podman by mounting the host\'s Podman socket This allows running containers inside the jail (useful for MCP agents)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand create" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand remove" -s f -l force -d 'Force removal without confirmation'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand remove" -l volume -d 'Remove associated volume (persistent data)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand remove" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand remove" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand remove" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand status" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand status" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand status" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand save" -s o -l output -d 'Output file path' -r -F
complete -c jail-ai -n "__fish_jail_ai_using_subcommand save" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand save" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand save" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -s b -l backend -d 'Backend type: \'podman\' (Linux) or \'container-app\' (macOS/apple-container)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -s i -l image -d 'Base image (e.g., localhost/jail-ai-env:latest, alpine:latest)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -s m -l mount -d 'Bind mount (format: source:target[:ro])' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -s p -l port -d 'Port mapping from host to container (format: host_port:container_port or host_port:container_port/protocol)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -s e -l env -d 'Environment variable (format: KEY=VALUE)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l memory -d 'Memory limit in MB' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l cpu -d 'CPU quota percentage (0-100)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l workspace-path -d 'Custom workspace path inside jail (default: /workspace)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l layers -d 'Force specific layers (comma-separated, e.g., "base,rust,python")' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l no-network -d 'Disable network access'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l host-network -d 'Use host networking (--network=host) instead of private networking Less secure but provides full access to host network services'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l no-workspace -d 'Skip auto-mounting current working directory to /workspace'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l agent-configs -d 'Mount all agent config directories'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l git-gpg -d 'Enable git and GPG configuration mapping'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l upgrade -d 'Upgrade: rebuild outdated layers and recreate container'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l cloud -d 'Include cloud provider layers (AWS + GCP tools)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l shell -d 'Start an interactive shell instead of running the agent command'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l isolated -d 'Use isolated project-specific images (workspace hash tag) instead of shared layer-based images'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l auth -d 'Open interactive shell for OAuth authentication (joins running container or starts stopped one)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l no-nix -d 'Skip nix layer (by default, nix takes precedence over other language layers)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l no-block-host -d 'Disable eBPF-based host blocking (allows connections to host IPs) [default: enabled]'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l podman -d 'Enable Podman-in-Podman by mounting the host\'s Podman socket This allows running containers inside the jail (useful for MCP agents)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -l tui -d 'Launch the TUI with a tab for the agent and a tab for an interactive shell'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -f -a "claude"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -f -a "claude-code-router"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -f -a "coderabbit"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -f -a "codex"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -f -a "copilot"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -f -a "cursor"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -f -a "gemini"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -f -a "jules"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -f -a "opencode"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -f -a "pi"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and not __fish_seen_subcommand_from claude claude-code-router coderabbit codex copilot cursor gemini jules opencode pi help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from claude" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from claude" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from claude" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from claude-code-router" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from claude-code-router" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from claude-code-router" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from coderabbit" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from coderabbit" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from coderabbit" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from codex" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from codex" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from codex" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from copilot" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from copilot" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from copilot" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from cursor" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from cursor" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from cursor" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from gemini" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from gemini" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from gemini" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from jules" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from jules" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from jules" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from opencode" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from opencode" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from opencode" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from pi" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from pi" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from pi" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from help" -f -a "claude"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from help" -f -a "claude-code-router"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from help" -f -a "coderabbit"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from help" -f -a "codex"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from help" -f -a "copilot"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from help" -f -a "cursor"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from help" -f -a "gemini"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from help" -f -a "jules"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from help" -f -a "opencode"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from help" -f -a "pi"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand agents; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand list" -s b -l backend -d 'Backend type (only \'podman\' is supported, kept for compatibility)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand list" -s c -l current -d 'Show only jails for current directory'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand list" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand list" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand list" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand clean-all" -s b -l backend -d 'Backend type (only \'podman\' is supported, kept for compatibility)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand clean-all" -s f -l force -d 'Force removal without confirmation'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand clean-all" -l volume -d 'Remove associated volumes (persistent data)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand clean-all" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand clean-all" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand clean-all" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand upgrade" -s i -l image -d 'Base image to upgrade to (e.g., localhost/jail-ai-env:latest, alpine:latest)' -r
complete -c jail-ai -n "__fish_jail_ai_using_subcommand upgrade" -s f -l force -d 'Force upgrade without confirmation'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand upgrade" -l all -d 'Upgrade all jails'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand upgrade" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand upgrade" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand upgrade" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand completions" -s v -l verbose -d 'Enable verbose logging'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand completions" -s q -l quiet -d 'Quiet mode (suppress INFO logs, only show warnings and errors)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand completions" -s h -l help -d 'Print help'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and not __fish_seen_subcommand_from create remove status save agents list clean-all upgrade completions help" -f -a "create" -d 'Create a new jail'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and not __fish_seen_subcommand_from create remove status save agents list clean-all upgrade completions help" -f -a "remove" -d 'Remove a jail'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and not __fish_seen_subcommand_from create remove status save agents list clean-all upgrade completions help" -f -a "status" -d 'Show jail status'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and not __fish_seen_subcommand_from create remove status save agents list clean-all upgrade completions help" -f -a "save" -d 'Save jail configuration to file'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and not __fish_seen_subcommand_from create remove status save agents list clean-all upgrade completions help" -f -a "agents" -d 'Run AI agents. Agent variants are auto-generated from agents/mod.rs'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and not __fish_seen_subcommand_from create remove status save agents list clean-all upgrade completions help" -f -a "list" -d 'List all jails'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and not __fish_seen_subcommand_from create remove status save agents list clean-all upgrade completions help" -f -a "clean-all" -d 'Stop and remove all jail-ai containers'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and not __fish_seen_subcommand_from create remove status save agents list clean-all upgrade completions help" -f -a "upgrade" -d 'Upgrade jail by recreating it with the latest image'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and not __fish_seen_subcommand_from create remove status save agents list clean-all upgrade completions help" -f -a "completions" -d 'Generate shell completions and print them to stdout'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and not __fish_seen_subcommand_from create remove status save agents list clean-all upgrade completions help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and __fish_seen_subcommand_from agents" -f -a "claude"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and __fish_seen_subcommand_from agents" -f -a "claude-code-router"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and __fish_seen_subcommand_from agents" -f -a "coderabbit"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and __fish_seen_subcommand_from agents" -f -a "codex"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and __fish_seen_subcommand_from agents" -f -a "copilot"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and __fish_seen_subcommand_from agents" -f -a "cursor"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and __fish_seen_subcommand_from agents" -f -a "gemini"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and __fish_seen_subcommand_from agents" -f -a "jules"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and __fish_seen_subcommand_from agents" -f -a "opencode"
complete -c jail-ai -n "__fish_jail_ai_using_subcommand help; and __fish_seen_subcommand_from agents" -f -a "pi"
