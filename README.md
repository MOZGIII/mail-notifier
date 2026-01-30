# Mail Notifier

Mail Notifier is a cross-platform application that monitors IMAP email mailboxes for unread message counts and provides notifications through multiple interfaces: system tray icon, terminal user interface (TUI), or command-line logging.

## Configuration

Create a YAML configuration file, e.g., `config.yml`. The application looks for configuration files in the following locations, in order of preference:

- `$XDG_CONFIG_DIR/mail-notifier/config.yaml`
- `$XDG_CONFIG_DIR/mail-notifier.yaml`
- `$HOME/.mail-notifier.yaml`
- `$HOME/.mail-notifier/config.yaml`
- `/etc/mail-notifier/config.yaml`

You can also set the `MAIL_NOTIFIER_CONFIG` environment variable to specify a custom path.

### Password Authentication

```yaml
servers:
  - name: example
    host: imap.example.com
    port: 993
    tls:
      mode: implicit
    login:
      username: username@example.com
      password:
        keyring: {}
    view_url: https://mail.example.com/
    mailboxes:
      - name: INBOX
```

### OAuth 2 Authentication (Gmail Example)

```yaml
oauth2_clients:
  gmail:
    client_id: your_google_client_id
    client_secret: your_google_client_secret
    token_url: https://oauth2.googleapis.com/token
    auth_url: https://accounts.google.com/o/oauth2/auth

servers:
  - name: gmail
    host: imap.gmail.com
    port: 993
    tls:
      mode: implicit
    oauth2_session:
      user: your_email@gmail.com
      oauth2_client: gmail
      oauth2_scopes: [https://mail.google.com/]
      oauth2_init_extra_params:
        access_type: offline
        prompt: consent
        login_hint: your_email@gmail.com
      keyring: {}
    view_url: https://gmail.com/
    mailboxes:
      - name: INBOX
```

## Usage

### System Tray

Run the tray application:

```bash
cargo run -p tray
```

This displays a system tray icon showing unread email counts.

For macOS-specific installation and usage instructions, see the [macOS tray README](macos/tray/README.md).

### Terminal UI

Run the TUI application:

```bash
cargo run -p tui
```

Provides an interactive terminal interface for monitoring mailboxes.

### Command-Line Logging

Run the CLI application:

```bash
cargo run -p cli
```

Logs mailbox updates to the console.

### Keyring Setup

For password authentication, store passwords in the system keyring:

```bash
echo "your_password" | cargo run -p keyring-set <server-name>
```

This writes the password to the keyring for the specified server.

### OAuth 2 Setup

For OAuth 2 authentication, first initialize:

```bash
cargo run -p oauth2-init <server-name>
```

This opens a browser for authentication and stores tokens in the keyring.
