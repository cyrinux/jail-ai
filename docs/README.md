# jail-ai Documentation

This directory contains the man pages for jail-ai.

## Man Page Files

- **jail-ai.1** - Traditional groff format man page (for `man` command)
- **jail-ai.1.md** - Markdown version of the man page (for web viewing)

## Viewing Man Pages Locally

To preview the man page without installing it system-wide:

```bash
# Using make
make view-man

# Or directly with man
man ./docs/jail-ai.1
```

## Installing Man Pages System-wide

To install the man page so you can access it with `man jail-ai` from anywhere:

```bash
# Install to /usr/local (default)
sudo make install-man

# Install to custom prefix (e.g., ~/.local)
make install-man PREFIX=~/.local

# Install with custom DESTDIR (for packaging)
make install-man DESTDIR=/tmp/jail-ai-package
```

After installation, you can view the man page with:

```bash
man jail-ai
```

## Uninstalling Man Pages

To remove the installed man page:

```bash
# Uninstall from /usr/local (default)
sudo make uninstall-man

# Uninstall from custom prefix
make uninstall-man PREFIX=~/.local
```

## Man Page Contents

The man pages document:

- All commands (create, remove, status, save, claude, copilot, cursor, gemini, codex, list, clean-all, upgrade, join)
- Global options (--verbose, --quiet)
- Common options (--backend, --image, --mount, --env, --memory, --cpu, etc.)
- Agent-specific options (--claude-dir, --copilot-dir, --cursor-dir, --gemini-dir, --codex-dir, --agent-configs, --git-gpg)
- Comprehensive examples for all use cases
- Files and directories used by jail-ai
- Environment variables
- Tools available in the default image
- Security considerations and best practices

## Updating Man Pages

When updating the man pages:

1. Edit **jail-ai.1** (groff format)
2. Edit **jail-ai.1.md** (markdown format)
3. Keep both files in sync
4. Update the date in the `.TH` header (groff) and footer (markdown)
5. Test locally with `make view-man`
6. Reinstall with `sudo make install-man` if needed

## Format Reference

### Groff Man Page Format

The groff man page uses traditional man page macros:

- `.TH` - Title header
- `.SH` - Section header
- `.TP` - Tagged paragraph (for options)
- `.B` - Bold text
- `.I` - Italic text
- `.BR` - Bold/Roman combination
- `.IP` - Indented paragraph

### Markdown Man Page

The markdown version provides a more readable format for:

- GitHub/GitLab documentation viewing
- Online documentation sites
- Easier editing and reviewing

## See Also

- [CLAUDE.md](../CLAUDE.md) - Development guide and project overview
- [Project Homepage](https://github.com/cyrinux/jail-ai)
- [Documentation](https://docs.rs/jail-ai)
