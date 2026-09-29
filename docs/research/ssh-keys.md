
Since you are building the tool itself, you do not need to rely on CLI helpers like bkt. 

You can create a centralized, forge-agnostic OAuth flow embedded directly into your application's setup wizard.

The ideal, frictionless onboarding flow for your less-technical users looks like this:

The Ideal Onboarding Flow

    Key Generation: Your application automatically runs ssh-keygen in the background (or uses a library to generate an Ed25519 key pair) specifically for your tool.
    
    Forge Selection: The user clicks a button: "Connect your Bitbucket / GitHub / GitLab Account".
    Browser Handshake: Your app routes them through a quick OAuth 2.0 flow. They log into their forge, click "Authorize App", and the browser automatically redirects back to your application.
    Silent Upload: Your app uses the temporary OAuth access token behind the scenes to push the public key to the forge's API.
    Success: The user is immediately ready to edit documentation. They never see an SSH key, a terminal, or an API settings page.

Forge API Reference for Key Provisioning

To implement this, you can write a clean abstraction layer in your codebase. When the OAuth handshake returns a token, you fire a single POST request to the respective forge's SSH key endpoint using the user's temporary bearer token.

1. Bitbucket Cloud

    OAuth Scope Needed: account:write
    Endpoint: POST https://bitbucket.org{selected_user}/ssh-keys
    Payload: [1, 2]
    json

    {
      "label": "MyDocumentationTool Key",
      "key": "ssh-ed25519 AAAAC3NzaC..."
    }

    Use code with caution.

2. GitHub

    OAuth Scope Needed: write:public_key
    Endpoint: POST https://github.com
    Payload:
    json

    {
      "title": "MyDocumentationTool Key",
      "key": "ssh-ed25519 AAAAC3NzaC..."
    }

    Use code with caution.

3. GitLab

    OAuth Scope Needed: api or write_repository
    Endpoint: POST https://gitlab.com
    Payload:
    json

    {
      "title": "MyDocumentationTool Key",
      "key": "ssh-ed25519 AAAAC3NzaC..."
    }

4. Gitolite / Self-Hosted Git (Fallback Script)

For self-hosted instances like Gitolite, you cannot rely on an interactive OAuth API. For these users, your tool can display a fallback screen. Give them a "Copy Public Key" button alongside a simple shell snippet they can hand to their DevOps/SysAdmin team:
bash

# Provide this for their IT admin to add to the gitolite-admin repo:
echo "ssh-ed25519 AAAAC3NzaC..." > keydir/user_name.pub
git add keydir/user_name.pub && git commit -m "Add SME key" && git push


Designing the Architecture

By implementing a single code interface (e.g., GitForgeProvider) with specific sub-classes for BitbucketProvider, GithubProvider, and GitlabProvider, you achieve your goal of a generalized solution. The tool hides all the technical complexity, making Git's collaboration mechanics completely invisible to your SMEs.


---

Rust is an excellent choice for this. Its strict type system is perfect for building a robust GitForge abstraction layer, and the reqwest and oauth2 crates will make handling multiple web forges highly manageable.
Here is a structural blueprint for how to implement the generalized key-provisioning system in Rust.
1. Cargo Dependencies
You will want to add these to your Cargo.toml:
toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
# Option for managing OAuth state effortlessly:
oauth2 = "4.4" 
Use code with caution.
2. The Generalized Forge Trait
You can define a uniform async interface. Your application logic only interacts with this trait, ignoring whether the underlying service is Bitbucket, GitHub, or GitLab.
rust
use async_trait::async_trait; // Requires `async-trait` crate, or use native async trait in Rust 1.75+
use serde::Serialize;

#[derive(Serialize)]
pub struct SshKeyPayload {
    pub title: String,
    pub key: String,
}

#[async_trait]
pub trait GitForge {
    /// Returns the exact API endpoint to POST the SSH key to
    fn ssh_key_endpoint(&self) -> &str;
    
    /// Translates the standard payload into the format the specific forge expects
    fn serialize_payload(&self, title: &str, public_key: &str) -> serde_json::Value;

    /// Uploads the generated public key using the user's temporary OAuth access token
    async fn upload_ssh_key(&self, token: &str, title: &str, public_key: &str) -> Result<(), reqwest::Error> {
        let client = reqwest::Client::new();
        let payload = self.serialize_payload(title, public_key);

        let response = client
            .post(self.ssh_key_endpoint())
            .bearer_auth(token)
            .header("User-Agent", "YourDocToolApp") // GitHub strictly requires a User-Agent
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            // Handle error logging or return custom error types here
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            eprintln!("Forge upload failed ({status}): {body}");
            // Return an error wrapper in real implementation
            Ok(()) 
        }
    }
}
Use code with caution.
3. Implementing the Bitbucket & GitHub Providers
Because Bitbucket and GitHub expect slightly different JSON keys (label vs title), your specific implementations handle that mapping locally.
rust
pub struct BitbucketProvider;

#[async_trait]
impl GitForge for BitbucketProvider {
    fn ssh_key_endpoint(&self) -> &str {
        // Note: For Bitbucket, if you don't know the exact username, 
        // you can often use the "/2.0/user/ssh-keys" endpoint (singular 'user') 
        // to associate it with whoever owns the bearer token.
        "https://bitbucket.org"
    }

    fn serialize_payload(&self, title: &str, public_key: &str) -> serde_json::Value {
        serde_json::json!({
            "label": title,
            "key": public_key
        })
    }
}

pub struct GithubProvider;

#[async_trait]
impl GitForge for GithubProvider {
    fn ssh_key_endpoint(&self) -> &str {
        "https://github.com"
    }

    fn serialize_payload(&self, title: &str, public_key: &str) -> serde_json::Value {
        serde_json::json!({
            "title": title,
            "key": public_key
        })
    }
}
Use code with caution.
4. Handling the OAuth Redirect inside a Rust Desktop/Local App
Since your users are running this tool locally, handling the temporary token retrieval seamlessly is critical:
• The Loopback approach: When the user clicks "Connect to Bitbucket", your Rust application spins up a tiny, temporary HTTP server on localhost (e.g., 127.0.0.1:8080) using a crate like axum or tiny_http.
• The Authorization redirect: Your app triggers the system browser to open Bitbucket's OAuth authorization page.
• The Catch: Once the user clicks "Allow", Bitbucket redirects the browser to http://127.0.0.... Your background Rust server catches that request, extracts the authorization code, swaps it for an access token, shuts down the local port, and seamlessly completes the SSH setup.
Are you building this as a desktop app (Tauri / iced), a CLI-based tool, or a web app? Knowing your UI approach will help narrow down the cleanest way to orchestrate that local OAuth redirect.

---

Yes, you must register your application with each Git forge (Atlassian/Bitbucket, GitHub, GitLab) to use their OAuth 2.0 flows.
When you register, the forge acts as the centralized identity provider. They will issue you a Client ID (which is safe to distribute publicly in your application) and a Client Secret (which requires special handling for desktop/CLI apps).
Here is how you navigate registration and architecture for a combined GPUI desktop and headless CLI tool in Rust:
1. The Desktop/CLI Challenge: Handling the "Client Secret"
Most OAuth flows expect a web server to safely hold a Client Secret. Because your app runs locally on a user's machine, you cannot securely hide a Client Secret inside your compiled Rust binary; someone could reverse-engineer it.
To solve this, each forge provides a specific path for desktop and CLI apps:
• GitHub & GitLab: They support PKCE (Proof Key for Code Exchange) or explicitly allow you to perform the OAuth exchange without a client secret for "Native/Public Applications".
• Bitbucket (Atlassian): Bitbucket requires you to check a specific box during registration. When creating your OAuth consumer in the Bitbucket Workspace settings, you must check the "This is a native application" box. This tells Bitbucket's authorization server to bypass the client secret requirement during the token exchange step, making it perfectly safe for your GPUI app to communicate with it directly.
2. What to Configure in Each Forge Developer Portal
When registering your app at Atlassian Developer or GitHub Settings, you will need to provide two critical pieces of information:
• The Callback URL (Redirect URI): Register a local loopback address like http://127.0.0. When your GPUI app or headless binary spins up its temporary background listener, this is where the forge will send the authorization code.
• The API Scopes: Only request exactly what your app needs so users aren't scared off by massive permission blocks.
	• Bitbucket: account:write (to post SSH keys) and repository:write (to manage documentation sync).
	• GitHub: write:public_key and repo.
3. Handling the Headless (CLI) Flow seamlessly
While a GPUI desktop app can easily spawn a browser, a headless CLI running on a remote server (like an SSH session to a dev box) cannot open a browser pointing to localhost.
To make your headless version work, you have two options:
1. The Copy-Paste Fallback: If your loopback server fails or detects a headless environment, print the forge's OAuth URL to the terminal. Instruct the user to open it on their local machine. After they click approve, the forge will redirect to a broken localhost page in their browser. Instruct them to copy the ?code=XYZ string out of their browser address bar and paste it back into your CLI prompt.
2. GitHub Device Flow: GitHub supports a unique Device Flow where the CLI prints a short code (e.g., ABCD-1234) and a URL (github.com). The user activates it on their phone or laptop, and the headless CLI polls GitHub's API until it succeeds. (Note: Bitbucket Cloud does not natively support the Device Flow protocol, so you will have to use the Copy-Paste fallback for Bitbucket).
Rust Implementation Tip (GPUI Compatibility)
Since you are using Zed's gpui crate, you can utilize gpui::AppContext::spawn to cleanly run your temporary OAuth loopback web server on a background thread without blocking the GPUI main UI thread. When the server intercepts the token, it can invoke a GPUI global action to update your UI state and transition the SME to their new workspace.
Would you like to see how to structure the native OAuth PKCE / Secret-less token exchange using the oauth2 crate in Rust, or are you ready to jump into registering the apps on the developer consoles?

---

To make your native Git operations work over SSH, the OAuth token you fetch will strictly be a short-lived key used for a one-time operation: uploading the user's public SSH key. Once that key is uploaded to the forge, your GPUI desktop app or headless CLI can completely drop the token and use standard native Git calls (via standard std::process::Command or a library like git2-rs/gix) to pull and push documentation seamlessly.
Here is how to implement the Native/Public OAuth flow (using PKCE for GitHub/GitLab, and secret-less exchange for Bitbucket) using the oauth2 crate in Rust.
1. The OAuth Token Exchange Implementation
This module provides a generalized engine that handles token acquisition without needing a client secret.
rust
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, 
    PkceCodeVerifier, RedirectUrl, TokenResponse, TokenUrl
};
use std::error::Error;

pub struct OAuthFlowManager {
    client: BasicClient,
    pkce_verifier: Option<PkceCodeVerifier>,
}

impl OAuthFlowManager {
    pub fn new(client_id: &str, auth_url: &str, token_url: &str) -> Self {
        let client = BasicClient::new(
            ClientId::new(client_id.to_string()),
            None, // Crucial: No Client Secret is provided here for native apps
            AuthUrl::new(auth_url.to_string()).unwrap(),
            Some(TokenUrl::new(token_url.to_string()).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::new("http://127.0.0".to_string()).unwrap());

        Self {
            client,
            pkce_verifier: None,
        }
    }

    /// Step 1: Generate the URL to open in the user's browser
    pub fn generate_auth_url(&mut self, use_pkce: bool) -> String {
        let mut auth_request = self.client.authorize_url(CsrfToken::new_random);
        
        // Scope definitions depend on the forge (e.g., Bitbucket: account:write)
        auth_request = auth_request.add_scope(oauth2::Scope::new("account:write".to_string()));

        if use_pkce {
            // PKCE is standard for GitHub/GitLab native apps
            let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
            auth_request = auth_request.set_pkce_challenge(pkce_challenge);
            self.pkce_verifier = Some(pkce_verifier);
        }

        let (url, _csrf_token) = auth_request.url();
        url.to_string()
    }

    /// Step 2: Swap the authorization code from the callback for an Access Token
    pub async fn exchange_code_for_token(&mut self, code_str: &str) -> Result<String, Box<dyn Error>> {
        let code = AuthorizationCode::new(code_str.to_string());
        let mut exchange_request = self.client.exchange_code(code);

        // If we generated a PKCE verifier (GitHub/GitLab), attach it to the exchange
        if let Some(verifier) = self.pkce_verifier.take() {
            exchange_request = exchange_request.set_pkce_verifier(verifier);
        }

        // Execute the HTTP exchange against the forge's token endpoint
        // NOTE: Since you are using GPUI/Tokio, run this inside an async context.
        let http_client = reqwest::Client::builder().build()?;
        let token_result = exchange_request
            .request_async(&http_client)
            .await?;

        Ok(token_result.access_token().secret().to_string())
    }
}
Use code with caution.
2. Orchestrating the Flow in GPUI vs. Headless CLI
To hook this engine up seamlessly to both your desktop app and your headless CLI, structure the application sequence as follows:
rust
// Pseudo-architecture for your App initialization
pub async fn run_onboarding(client_id: &str, is_headless: bool) {
    // 1. Generate SSH Key Pair in background if it doesn't exist
    // (e.g., save to ~/.config/yourtool/id_ed25519)
    let public_key_data = "ssh-ed25519 AAAAC3NzaC..."; 

    // 2. Initialize our manager (Example: Bitbucket URLs)
    let mut manager = OAuthFlowManager::new(
        client_id,
        "https://bitbucket.org",
        "https://bitbucket.org"
    );

    // Bitbucket uses secretless native flow instead of strict SHA256 PKCE
    let use_pkce = false; 
    let auth_url = manager.generate_auth_url(use_pkce);

    // 3. Direct the User to Authenticate
    let auth_code = if is_headless {
        println!("Please log in by opening this link in your browser:\n\n{}\n", auth_url);
        print!("Enter the 'code' parameter from the resulting URL: ");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
    } else {
        // In GPUI, trigger a browser launch cleanly:
        let _ = open::that(&auth_url); // using the `open` crate
        
        // Spin up a brief background listener on 127.0.0.1:8080 to catch the token
        // In GPUI, you'd execute this via cx.background_executor().spawn(...)
        let intercepted_code = listen_for_local_callback().await;
        intercepted_code
    };

    // 4. Complete the exchange and upload the key
    if let Ok(access_token) = manager.exchange_code_for_token(&auth_code).await {
        // Trigger your trait implementation (e.g., BitbucketProvider) to upload
        // Once this returns Ok(()), your app is fully authenticated over native Git SSH!
        println!("SSH key successfully provisioned via API.");
    }
}

async fn listen_for_local_callback() -> String {
    // A simple tiny_http or axum single-request listener goes here...
    "captured_code_from_url".to_string()
}
Use code with caution.
Configuration Parameters per Forge
When initializing OAuthFlowManager::new, pass the following configurations depending on which forge the user selects:
Forge	auth_url	token_url	use_pkce
Bitbucket Cloud	https://bitbucket.org	https://bitbucket.org	false (Check "Native App" in console) Atlassian Support Docs
GitHub	https://github.com	https://github.com	true GitHub Developer Docs
GitLab	https://gitlab.com	https://gitlab.com	true GitLab Developer Docs
Would you like help writing the single-request loopback server logic (using a lightweight crate like tiny_http) to intercept the auth code without introducing heavy web framework dependencies?

---

To capture the authorization code seamlessly in your GPUI desktop app, you want a lightweight background listener that starts up, grabs the single incoming GET request from the browser redirect, and immediately shuts down.
Using the tiny_http crate is the perfect choice here. It relies entirely on standard library networking and avoids adding a massive web framework like Axum or Actix to your Rust binary.
1. Add Dependency
Add this to your Cargo.toml:
toml
[dependencies]
tiny_http = "0.12"
url = "2.5" # Excellent crate for safely parsing query strings
Use code with caution.
2. The Loopback Listener Implementation
Here is the production-ready code to parse and intercept the ?code=XYZ parameter. It includes a basic HTML response so the user's browser displays a clean "Success" message instead of a broken page error.
rust
use std::error::Error;
use tiny_http::{Response, Server};
use url::Url;

/// Spawns a temporary local server that blocks until it captures an OAuth code.
pub fn listen_for_oauth_code(port: u16) -> Result<String, Box<dyn Error>> {
    // Bind specifically to loopback 127.0.0.1 for security
    let bind_addr = format!("127.0.0.1:{}", port);
    let server = Server::http(&bind_addr)
        .map_err(|e| format!("Failed to bind to local port {}: {}", port, e))?;

    // Block until the first incoming request arrives (the OAuth redirect)
    if let Some(request) = server.incoming_requests().next() {
        // Construct a full fake URL base so the `url` crate can parse the query parameters
        let request_url = format!("http://localhost{}{}", bind_addr, request.url());
        let parsed_url = Url::parse(&request_url)?;

        // Search for the "code=" parameter inside the redirect string
        let auth_code = parsed_url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.into_owned());

        if let Some(code) = auth_code {
            // Provide a friendly visual confirmation to the SME in their browser window
            let html_response = r#"
                <!DOCTYPE html>
                <html>
                <head><style>body { font-family: sans-serif; text-align: center; margin-top: 10%; color: #333; }</style></head>
                <body>
                    <h2>✓ Successfully Authenticated!</h2>
                    <p>You can close this tab now and return to your application.</p>
                </body>
                </html>
            "#;

            let response = Response::from_string(html_response)
                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap())
                .with_status_code(200);
            
            let _ = request.respond(response);
            
            // Returning drops the `server` object, closing the port immediately
            return Ok(code);
        } else {
            // Send error fallback response to browser
            let error_response = Response::from_string("Authentication failed: 'code' missing.")
                .with_status_code(400);
            let _ = request.respond(error_response);
        }
    }

    Err("Server was closed before an auth code could be extracted.".into())
}
Use code with caution.
3. Integrating with GPUI Async Context
Because tiny_http uses standard synchronous blocking calls to wait for network traffic, you should never run listen_for_oauth_code directly on GPUI's main thread (which would freeze the entire UI window).
Instead, leverage GPUI’s built-in background task threadpool via cx.background_executor(). Once the background thread grabs the token, it safely updates your app state using cx.update():
rust
// Inside a GPUI action or click handler:
pub fn handle_connect_button(cx: &mut AppContext) {
    let port = 8080;
    
    // 1. Kick off the blocking loopback server on a threadpool background thread
    cx.background_executor().spawn(async move {
        match listen_for_oauth_code(port) {
            Ok(code) => {
                // 2. Safely bounce back onto the GPUI main thread to swap code and upload key
                cx.update(|cx| {
                    println!("Successfully captured OAuth Code: {}", code);
                    // Trigger your token exchange code and upload keys here
                    // e.g., your_state.complete_onboarding(code, cx);
                }).ok();
            }
            Err(e) => {
                eprintln!("Error listening for OAuth callback: {:?}", e);
            }
        }
    }).detach(); // .detach() allows it to run independently in the background
}
Use code with caution.
This ensures a smooth workflow: the user clicks the login button in your GPUI interface, their browser pops open instantly, they authorize the app, the browser immediately changes to a checkmark screen, and your application's state seamlessly transitions to setting up native Git SSH in the background.
Would you like help mapping out the JSON response parsing step for Bitbucket's token exchange endpoint, or are you ready to structure the background SSH key generation (ssh-keygen) code next?

---

To generate Ed25519 SSH keys natively inside your Rust binary without shelling out to ssh-keygen, you can use the cryptographically secure ssh-key crate.
This crate manages the entire cryptographic lifecycle: it generates secure random private bytes, derives the public components, and encodes them into standard OpenSSH file formats (id_ed25519 and id_ed25519.pub) that your user's system SSH client can read.
1. Add Dependencies
Add these crates to your Cargo.toml. We include rand_core to safely tap into the host operating system's hardware random number generator (CSPRNG):
toml
[dependencies]
ssh-key = { version = "0.6", features = ["alloc", "crypto", "ed25519"] }
rand_core = { version = "0.6", features = ["getrandom"] }
Use code with caution.
2. Native Key Generation Implementation
The following function generates an Ed25519 key pair natively, wraps an optional comment around it (useful for identifying keys made by your application), and formats both the private OpenSSH structure and public key string.
rust
use rand_core::OsRng;
use ssh-key::{Algorithm, LineEnding, PrivateKey};
use std::error::Error;

pub struct GeneratedKeyPair {
    /// The string to upload to Bitbucket/GitHub (e.g., "ssh-ed25519 AAAAC3...")
    pub public_key_openssh: String,
    /// The string to save locally to ~/.ssh/id_ed25519 or an app data folder
    pub private_key_openssh: String,
}

pub fn generate_native_ed25519_key(comment: &str) -> Result<GeneratedKeyPair, Box<dyn Error>> {
    // 1. Generate an Ed25519 keypair using the OS secure random number generator
    let private_key_crypto = ssh_key::private::Ed25519Keypair::random(&mut OsRng);
    
    // 2. Wrap it into an OpenSSH PrivateKey structure along with a descriptive label
    let private_key = PrivateKey::new(private_key_crypto, comment)?;

    // 3. Extract and format the Public Key to standard OpenSSH string format
    let public_key = private_key.public_key();
    let public_key_openssh = public_key.to_openssh()?;

    // 4. Format the Private Key block using standard Unix OpenSSH line endings
    let private_key_openssh = private_key.to_openssh(LineEnding::LF)?.to_string();

    Ok(GeneratedKeyPair {
        public_key_openssh,
        private_key_openssh,
    })
}
Use code with caution.
3. Saving the Key Responsibly (File Permissions)
When storing the generated private key string onto the user's hard drive (like ~/.ssh/your_app_ed25519), Unix systems require specific file permissions. If the private key is readable by other users on the system, git over SSH will reject the key with a warning.
You can enforce the classic chmod 600 (Owner Read/Write Only) natively in Rust using target-specific extensions:
rust
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn save_keys_to_disk(path: &Path, keys: &GeneratedKeyPair) -> std::io::Result<()> {
    // Create or truncate the private key file
    let mut private_file = File::create(path)?;
    
    // On Unix systems (Linux / macOS), strictly enforce owner-only permissions (0o600)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = private_file.metadata()?.permissions();
        perms.set_mode(0o600); 
        private_file.set_permissions(perms)?;
    }

    private_file.write_all(keys.private_key_openssh.as_bytes())?;

    // Create the public key counterpart file (0o644 is standard but default create is usually fine)
    let pub_path = path.with_extension("pub");
    let mut public_file = File::create(pub_path)?;
    public_file.write_all(keys.public_key_openssh.as_bytes())?;

    Ok(())
}
Use code with caution.
4. Directing Git Operations to Use This Specific Key
Since this is a dedicated key built directly inside your project management tool, you don't want to pollute or rely on the user's global configuration files.
When your application fires off background Git operations (using standard processes), you can cleanly inject the environment variable GIT_SSH_COMMAND to instruct git exactly which key to leverage:
rust
use std::process::Command;

pub fn execute_git_sync(repo_path: &Path, identity_file_path: &Path) -> std::io::Result<()> {
    // -o IdentitiesOnly=yes ensures it doesn't try standard user keys if yours fails
    let ssh_command = format!(
        "ssh -i {} -o IdentitiesOnly=yes", 
        identity_file_path.to_string_lossy()
    );

    Command::new("git")
        .current_dir(repo_path)
        .arg("pull")
        .env("GIT_SSH_COMMAND", ssh_command) // Injects isolated key location
        .spawn()?
        .wait()?;

    Ok(())
}
Use code with caution.
By generating keys via ssh-key, catching OAuth codes via tiny_http, and enforcing custom GIT_SSH_COMMAND pathways, you have created a completely self-contained system. Non-technical users can interact safely with remote repositories over standard SSH without ever interacting with raw keys or terminal commands.
Would you like to explore how to safely read existing keys if a user prefers to reuse an environment key, or should we look at creating a GPUI loading state indicator for when this generation and setup process runs?

---

Integrating native SSH access with git2 (the Rust bindings for libgit2) is straightforward, but it handles authentication differently than a command-line git binary.
Because libgit2 is an embedded library, it does not respect the GIT_SSH_COMMAND environment variable. Instead, you inject your custom native keys directly into the network transport layer using git2::RemoteCallbacks right before performing a fetch, pull, clone, or push operation.
Here is how to wire up your generated native keys directly into libgit2 operations.
1. The Key Authentication Callback Blueprint
When libgit2 initiates an SSH handshake with Bitbucket or GitHub, it invokes a callback asking for credentials. You satisfy this by providing a git2::Cred::ssh_key reference pointing directly to the file path of your natively generated private key.
rust
use git2::{RemoteCallbacks, FetchOptions, Repository, Cred};
use std::path::Path;

/// Configures remote callbacks to use our isolated app-specific SSH key pair
pub fn create_ssh_callbacks<'a>(private_key_path: &'a Path) -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    
    callbacks.credentials(move |_url, username_from_url, _allowed_types| {
        // Most git forges use the unified username "git" for all SSH access
        let username = username_from_url.unwrap_or("git");
        
        // Pass the paths to your private key file. 
        // Pass None for the public key path as libgit2 can derive it from the private key file.
        // Pass None for the passphrase since our native generator didn't specify one.
        Cred::ssh_key(
            username,
            None,
            private_key_path,
            None
        )
    });

    callbacks
}
Use code with caution.
2. Performing a git2 Sync Operation (e.g., Fetch/Pull)
When syncing documentation updates for your non-technical SMEs, attach these credentials directly into your network operations payload:
rust
pub fn fetch_remote_repo(repo_path: &Path, private_key_path: &Path) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    let mut remote = repo.find_remote("origin")?;

    // 1. Initialize our SSH credentials callback
    let callbacks = create_ssh_callbacks(private_key_path);

    // 2. Wrap those callbacks into FetchOptions
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    // 3. Execute the network transfer cleanly over libgit2
    // This triggers your callback under the hood to perform the cryptographically secure handoff
    remote.fetch(&["refs/heads/main:refs/remotes/origin/main"], Some(&mut fetch_options), None)?;

    Ok(())
}
Use code with caution.
3. Cloning a Repository from Scratch
For your onboarding wizard, you can perform a complete clone using the exact same strategy:
rust
use git2::build::RepoBuilder;

pub fn clone_new_repo(url: &str, target_path: &Path, private_key_path: &Path) -> Result<Repository, git2::Error> {
    let callbacks = create_ssh_callbacks(private_key_path);
    
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_options);

    // Clones the repo cleanly into the target directory using the new key
    let repo = builder.clone(url, target_path)?;
    Ok(repo)
}
Use code with caution.
Key Architectural Benefit
Using libgit2 with explicit callbacks fits beautifully with your GPUI desktop app architecture:
• Zero Host Pollution: Your application does not require the user to have standard command-line tools (git or ssh) installed on their computer. Everything runs natively within your binary.
• Isolated Environment: You bypass global configuration quirks like the system ~/.ssh/config file, which prevents corporate SSH configurations or other personal keys from interfering with your application's documentation sync layer.
Would you like to explore how to implement parallel asynchronous progress mapping (so you can show a visual progress bar inside your GPUI interface as libgit2 transfers bytes from Bitbucket), or are you ready to bind the OAuth exchange payload next?

---

To finalize the token exchange process, your application sends an HTTP POST payload to the forge's token endpoint.
Because your tool is a native application (Desktop/CLI), the payload structure differs significantly depending on whether the forge supports PKCE (GitHub/GitLab) or Secretless Native Clients (Bitbucket Cloud). [1] (https://www.oauth.com/oauth2-servers/oauth-native-apps/)
1. The Raw Token Exchange Payloads
When the oauth2 crate executes .request_async(), it serializes one of two payload formats under the hood using an application/x-www-form-urlencoded body:
A. The Bitbucket Cloud Payload (No Secret, No PKCE)
Bitbucket Cloud expects a native desktop application to bypass the client_secret entirely during token exchange, provided you have checked the "This is a native application" box in the Bitbucket Workspace Settings. [1] (https://community.developer.atlassian.com/t/oauth-2-0-with-proof-key-for-code-exchange-pkce/80173)
• Endpoint: POST https://bitbucket.org/site/oauth2/access_token
• Payload Structure: [1] (https://www.servicenow.com/docs/r/it-service-management/devops-change-velocity/set-up-oauth-2-0-authorization-code.html)
http
POST /site/oauth2/access_token HTTP/1.1
Host: bitbucket.org
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code&
client_id=YOUR_PUBLIC_CLIENT_ID&
code=CAPTURED_AUTH_CODE&
redirect_uri=http%3A%2F%2F127.0.0.1%3A8080%2Fcallback
Use code with caution.
B. The GitHub / GitLab Payload (Using PKCE)
GitHub and GitLab do not use a dedicated "Native App" toggle. Instead, they enforce PKCE (Proof Key for Code Exchange). Your app sends the cryptographically verified raw string key (code_verifier) that matches the hashed challenge sent earlier during the browser step. [1] (https://github.com/danielkov/arctic-oauth), [2] (https://www.oauth.com/oauth2-servers/oauth-native-apps/), [3] (https://dev.to/xetri/building-a-google-oauth-cli-in-rust-with-pkce-and-surviving-the-borrow-checker-3cij)
• Endpoint: POST https://github.com
• Payload Structure: [1] (https://leapcell.io/blog/building-a-secure-rust-backend-with-oauth-2-0-authorization-code-flow)
http
POST /login/oauth/access_token HTTP/1.1
Host: github.com
Content-Type: application/x-www-form-urlencoded
Accept: application/json

grant_type=authorization_code&
client_id=YOUR_PUBLIC_CLIENT_ID&
code=CAPTURED_AUTH_CODE&
code_verifier=dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk&
redirect_uri=http%3A%2F%2F127.0.0.1%3A8080%2Fcallback
Use code with caution.
2. Handling the JSON Response Payload in Rust
All modern forges return a JSON payload upon a successful exchange. You will want to deserialize this token data to grab the access_token (and optionally the refresh_token if you want to extend access without prompting the user again). [1] (https://support.atlassian.com/user-management/docs/create-oauth-2-0-credential-for-service-accounts/), [2] (https://www.youtube.com/watch?v=rb_SZE6Sh20&t=224)
Here is how to strongly type the response using serde:
rust
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ForgeTokenResponse {
    /// The temporary token used to authorize the SSH key upload request
    pub access_token: String,
    
    /// The string token type, typically "bearer" or "Bearer"
    pub token_type: String,
    
    /// Lifetime of the access token in seconds (usually 3600 / 1 hour)
    pub expires_in: u64,
    
    /// Optional refresh token used to request a new access token seamlessly
    pub refresh_token: Option<String>,
    
    /// List of scopes returned by the forge confirming what access was granted
    pub scope: Option<String>,
}
Use code with caution.
3. Combining Everything into a Clean Module
The oauth2 crate abstracts the payload assembly automatically. By integrating the token deserialization with the token receiver logic, your GPUI/CLI wrapper can return a ready-to-use token instantly:
rust
use reqwest::Client;
use std::error::Error;

pub async fn exchange_code_for_forge_token(
    client_id: &str,
    code: &str,
    token_url: &str,
    code_verifier: Option<&str>, // Passed if using PKCE (GitHub/GitLab)
) -> Result<ForgeTokenResponse, Box<dyn Error>> {
    let client = Client::new();
    
    // Construct the urlencoded key-value pairs manually or via standard library formats
    let mut params = vec![
        ("grant_type", "authorization_code"),
        ("client_id", client_id),
        ("code", code),
        ("redirect_uri", "http://127.0.0"),
    ];

    // Explicitly push the PKCE verification string if handling GitHub or GitLab
    if let Some(verifier) = code_verifier {
        params.push(("code_verifier", verifier));
    }

    let response = client
        .post(token_url)
        .header("Accept", "application/json") // Forces GitHub to return structured JSON
        .header("User-Agent", "YourDocToolApp")
        .form(&params) // Form-encodes the payload into application/x-www-form-urlencoded
        .send()
        .await?;

    if response.status().is_success() {
        let token_data: ForgeTokenResponse = response.json().await?;
        Ok(token_data)
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Token exchange failed with status {}: {}", status, body).into())
    }
}
Use code with caution.
Next Steps For Your Architecture
1. Once this function yields Ok(token_data), pass token_data.access_token directly to the upload_ssh_key trait method we wrote earlier.
2. After the forge accepts the SSH key payload, wipe the token data memory securely (drop(token_data)).
3. Move instantly to your libgit2 clone/fetch setup utilizing the natively created files.
Would you like to build out the JSON error parsing payload engine next to gracefully catch scenarios where a user cancels the authorization process in the browser window?

---

When a user rejects or cancels your app's authorization request in their browser, or if the authorization code expires, the Git forge will not issue a token. Instead, they redirect the browser to your loopback server with specific URL error parameters, or return a structured JSON error body from their token endpoint.
To keep your GPUI desktop app stable and give the non-technical subject matter expert clear instructions (e.g., "Connection canceled by user" instead of an obscure network crash), you need to parse these error payloads cleanly.
1. Handling Browser-Side Cancellations (The Loopback Stage)
If the user clicks "Cancel" or "Deny" on the Bitbucket or GitHub web interface, the forge redirects their browser back to your tiny_http loopback listener (http://127.0.0), but replaces ?code= with ?error= and ?error_description=.
Let's update your tiny_http loopback parser to intercept these parameters dynamically:
rust
use std::error::Error;
use tiny_http::{Response, Server};
use url::Url;

#[derive(Debug)]
pub enum OAuthRedirectResult {
    Success(String),
    Canceled { error: String, description: String },
}

pub fn listen_for_oauth_callback(port: u16) -> Result<OAuthRedirectResult, Box<dyn Error>> {
    let bind_addr = format!("127.0.0.1:{}", port);
    let server = Server::http(&bind_addr).map_err(|e| format!("Port bind failure: {}", e))?;

    if let Some(request) = server.incoming_requests().next() {
        let request_url = format!("http://localhost{}{}", bind_addr, request.url());
        let parsed_url = Url::parse(&request_url)?;
        let params: std::collections::HashMap<_, _> = parsed_url.query_pairs().into_owned().collect();

        // 1. Check if the forge passed back an error parameter instead of a code
        if let Some(error_type) = params.get("error") {
            let description = params.get("error_description")
                .cloned()
                .unwrap_or_else(|| "No further details provided.".to_string());

            let html_error = format!(
                "<!DOCTYPE html><html><body style='font-family:sans-serif; text-align:center; margin-top:10%; color:#d32f2f;'>
                 <h2>✕ Connection Canceled</h2><p>{}</p></body></html>", 
                description
            );
            let _ = request.respond(Response::from_string(html_error).with_status_code(200));

            return Ok(OAuthRedirectResult::Canceled {
                error: error_type.clone(),
                description,
            });
        }

        // 2. Standard success pathway
        if let Some(code) = params.get("code") {
            let html_success = "<!DOCTYPE html><html><body style='font-family:sans-serif; text-align:center; margin-top:10%; color:#2e7d32;'>
                                <h2>✓ Success</h2><p>You can close this tab now.</p></body></html>";
            let _ = request.respond(Response::from_string(html_success).with_status_code(200));
            return Ok(OAuthRedirectResult::Success(code.clone()));
        }
    }

    Err("Server closed before processing the callback response.".into())
}
Use code with caution.
2. Handling Token-Exchange Errors (The API Stage)
If your app captures a code but the code is invalid or expired, the backend token POST endpoint will return a 400 Bad Request containing a standard OAuth 2.0 JSON payload.
Define this strongly-typed struct to safely parse the server's complaints:
rust
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ForgeErrorPayload {
    /// The high-level error identifier (e.g., "invalid_grant", "unauthorized_client")
    pub error: String,
    
    /// A human-readable description explaining exactly why the token was rejected
    pub error_description: Option<String>,
    
    /// An optional documentation URI pointing to how to resolve the error configuration
    pub error_uri: Option<String>,
}
Use code with caution.
3. Integrating Error Decoding with your HTTP Client
When your token exchange request yields an error status code, use serde to read the payload before throwing a blanket network failure. This gives your GPUI application surface engine the context it needs to render a custom prompt:
rust
use reqwest::{Client, StatusCode};

// Inside your token exchange method...
let response = client
    .post(token_url)
    .header("Accept", "application/json")
    .form(&params)
    .send()
    .await?;

match response.status() {
    StatusCode::OK => {
        let token_data: ForgeTokenResponse = response.json().await?;
        Ok(token_data)
    }
    StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED => {
        // Attempt to parse structured error definitions
        if let Ok(error_payload) = response.json::<ForgeErrorPayload>().await {
            let message = error_payload.error_description
                .unwrap_or_else(|| error_payload.error.clone());
            Err(format!("Forge Rejected Request: {}", message).into())
        } else {
            Err("Authentication endpoint returned an unparseable malformed error.".into())
        }
    }
    status => {
        Err(format!("Unexpected server network response status: {}", status).into())
    }
}
Use code with caution.
Common Error States to Plan for in your GPUI UI
When building out your workspace setup notifications, maps these error patterns to friendly feedback:
• access_denied: The SME hit "Cancel" in their browser window. Safely reset your GPUI screen back to the start wizard without printing a panic message.
• invalid_grant: The user took too long to complete the browser steps, and the code expired. Prompt them with a single click button: "Retry Connection".
Now that your auth infrastructure handles setup, token capture, native key orchestration, and failure state handling, would you like to review how to build out a structured Bitbucket API Client payload specifically for uploading the public key string to their user account?

---

To upload the public SSH key to Bitbucket Cloud, your Rust application will send an authenticated POST request to the Bitbucket REST API. [1] (https://developer.atlassian.com/cloud/bitbucket/rest/api-group-ssh/)
When executing this step using an OAuth 2.0 access token, Bitbucket lets you bypass looking up the user's specific UUID or account name. You can simply hit the unified /2.0/user/ssh-keys endpoint, and Bitbucket automatically associates the key with whichever user profile authorized your app. [1] (https://community.atlassian.com/forums/Bitbucket-questions/How-can-I-use-the-Bitbucket-API-to-add-an-SSH-key-to-a-Workspace/qaq-p/1836286)
1. Structuring the JSON Payloads
Bitbucket expects a JSON payload containing a label and the key. Upon success, it returns a subset of fields (like the resource uuid) which you can ignore or store locally. [1] (https://stackoverflow.com/questions/74814236/programmatically-add-an-access-key-to-a-bitbucket-repo-bitbucket-cloud-api), [2] (https://community.atlassian.com/forums/Bitbucket-questions/Is-there-an-API-call-for-adding-SSH-keys-to-a-BitBucket-repo/qaq-p/691282)
rust
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct BitbucketSshKeyRequest {
    /// A human-readable name for the key in the user's account configuration
    pub label: String,
    /// The exact public key string (e.g., "ssh-ed25519 AAAAC3NzaC...")
    pub key: String,
}

#[derive(Deserialize, Debug)]
pub struct BitbucketSshKeyResponse {
    /// Bitbucket's unique identifier for this specific uploaded key
    pub uuid: String,
    pub label: String,
    pub comment: Option<String>,
}
Use code with caution.
2. Building the Client Upload Logic
Here is the implementation using reqwest. This matches the generalized GitForge trait framework we structured earlier:
rust
use reqwest::{Client, StatusCode};
use std::error::Error;

pub async fn upload_key_to_bitbucket(
    access_token: &str,
    label: &str,
    public_key_string: &str,
) -> Result<BitbucketSshKeyResponse, Box<dyn Error>> {
    let client = Client::new();

    let payload = BitbucketSshKeyRequest {
        label: label.to_string(),
        key: public_key_string.trim().to_string(),
    };

    // The unified /user endpoint routes directly to the token bearer's account
    let endpoint = "https://bitbucket.org";

    let response = client
        .post(endpoint)
        .bearer_auth(access_token) // Sets the "Authorization: Bearer <token>" header
        .json(&payload)
        .send()
        .await?;

    match response.status() {
        StatusCode::CREATED | StatusCode::OK => {
            let success_data: BitbucketSshKeyResponse = response.json().await?;
            Ok(success_data)
        }
        StatusCode::BAD_REQUEST => {
            let body = response.text().await?.unwrap_or_else(|| "Malformed request layout".to_string());
            Err(format!("Bitbucket rejected key syntax: {}", body).into())
        }
        StatusCode::CONFLICT => {
            // This occurs if this exact SSH key string is already linked to their account
            Err("This SSH key is already registered to your Bitbucket account.".into())
        }
        StatusCode::UNAUTHORIZED => {
            Err("OAuth access token has expired or is invalid.".into())
        }
        status => {
            Err(format!("Bitbucket returned unexpected status code: {}", status).into())
        }
    }
}
Use code with caution.
The End-to-End Orchestration Loop
Now your application has all the distinct primitives needed to run an entirely native setup wizard for non-technical users:
[ SME Click Setup ]
        │
        ▼
 1. Generate Ed25519 Keys Natively (ssh-key + OsRng) ──► Saved cleanly to isolated App Data Dir
        │
        ▼
 2. Spin up Loopback Server (tiny_http) & open Browser ──► App catches code or "Access Denied" error
        │
        ▼
 3. POST Code Exchange ──► Trade secret-less Code payload for an API Access Token
        │
        ▼
 4. POST Bitbucket Upload ──► Push `BitbucketSshKeyRequest` to `/2.0/user/ssh-keys`
        │
        ▼
 5. Fire libgit2 Operations ──► Run `.fetch()` / `.clone()` referencing the local file handle directly
Would you like help designing the GPUI view logic to handle the state transitions (e.g., showing a processing spinner during key generation, turning into a "Check your browser" state, and finalizing with a success state)?
