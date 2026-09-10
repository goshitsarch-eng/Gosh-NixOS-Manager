app-title = NixOS Toolkit
app-comment = A declarative NixOS system management tool
view = View
about = About
quit = Quit
refresh = Refresh
repository = Repository

nav-onboarding = Getting Started
nav-profiles = Desktop Profiles
nav-bundles = Software Bundles
nav-packages = Custom Packages
nav-system = System Settings
nav-hardware = Hardware
nav-network = Network
nav-services = Services
nav-generations = Generations
nav-maintenance = Maintenance
nav-apply = Apply Changes

page-onboarding = Getting Started
page-profiles = Desktop Profiles
page-bundles = Software Bundles
page-packages = Custom Packages
page-system = System Settings
page-hardware = Hardware
page-network = Network
page-services = Services
page-generations = Generations
page-maintenance = Maintenance
page-apply = Apply Changes

page-onboarding-title = Welcome to NixOS Toolkit
page-onboarding-desc = This tool helps you manage your NixOS configuration declaratively. Follow the steps below to complete the one-time setup.

page-profiles-title = Desktop Environment
page-profiles-desc = Select a desktop environment profile. You can only have one active at a time.

page-bundles-title = Software Bundles
page-bundles-desc = Enable bundles and expand to customize individual packages.

page-packages-title = Custom Packages
page-packages-desc = Add individual packages from nixpkgs. Paste package names from search.nixos.org.

page-system-title = System Settings
page-system-desc = Configure system-level settings. These will be applied through NixOS configuration.

page-hardware-title = Hardware Configuration
page-hardware-desc = Configure graphics drivers, audio, bluetooth, and power management settings.

page-network-title = Network & Security
page-network-desc = Configure firewall rules, SSH access, and VPN settings.

page-services-title = System Services
page-services-desc = Enable or disable common system services. Changes require a system rebuild.

page-generations-title = System Generations
page-generations-desc = View, manage, and rollback NixOS system generations. Each generation represents a complete system configuration that you can boot into.

page-maintenance-title = System Maintenance
page-maintenance-desc = Clean up disk space and maintain your NixOS system.

page-apply-title = Apply Changes
page-apply-desc = Review your configuration and apply changes to the system.

onboarding-status = System Status
onboarding-not-nixos = Not NixOS
onboarding-not-nixos-sub = This tool only works on NixOS systems
onboarding-nixos-detected = NixOS Detected
onboarding-integration = Integration
onboarding-version = Version
onboarding-version-unknown = unknown
onboarding-setup = One-Time Setup
onboarding-setup-desc = Follow these steps to integrate the toolkit with your NixOS configuration

integration-integrated = Integrated
integration-not-integrated = Not integrated
integration-unknown = Unknown

menu-file = File
menu-view = View
menu-refresh = Refresh
menu-quit = Quit
menu-about = About NixOS Toolkit

banner-not-nixos = Not running on NixOS
banner-integrated = Integrated - Ready to apply changes
banner-setup-required = Setup required - See Getting Started
banner-state-helper-comm = Failed to communicate with helper
banner-state-read-error = State read error: { $message }
banner-state-timeout = State read timeout
banner-state-spawn = Could not load saved state - using defaults
banner-arm-bundles = Running on ARM64 - some packages may not be available
banner-arm-hardware = ARM64: NVIDIA drivers and Intel Thermald are not available

copy-snippet = Copy Snippet
open-nixos-dir = Open /etc/nixos
verify-integration = Verify Integration
add = Add
refresh-preview = Refresh Preview
refresh-disk = Refresh Disk Info
refresh-disk-busy = Calculating…
apply-changes = Apply Changes
apply-changes-busy = Applying…
dry-run = Dry Run
dry-run-busy = Building…
dry-run-tooltip = Build configuration without activating
helper-missing-action = Privileged helper not found. Install nixos-toolkit-helper on the host.
rollback-previous = Rollback to Previous
remove-package = Remove package
run-action = Run this action
switch-generation = Switch to this generation
delete-generation = Delete this generation
generation-current = Current
loading = Loading...
calculating = Calculating…

profiles-available = Available Profiles
profiles-preview = Preview
profiles-preview-desc = Nix configuration that will be generated
profiles-preview-placeholder = # Select a profile to see preview

bundles-available = Available Bundles
bundles-available-desc = Click to expand and customize packages
bundles-selected = Selected Packages
bundles-none-selected = No packages selected
bundles-package-count = { $count } packages

packages-add = Add Package
packages-add-desc = Paste package names like 'zed-editor' or 'pkgs.zed-editor' or even 'environment.systemPackages = [ pkgs.zed ];'
packages-placeholder = Package name (e.g., zed-editor, htop, neofetch)
packages-installed = Installed Custom Packages
packages-installed-desc = Packages will be installed when you click Apply
packages-empty = No custom packages
packages-empty-desc = Add packages above to get started

system-appearance = Appearance
system-style = Style
system-style-desc = Choose application color scheme
style-system = System
style-light = Light
style-dark = Dark
system-identity = Network Identity
system-hostname = Hostname
system-hostname-placeholder = nixos
system-hostname-note = Note
system-hostname-note-desc = Hostname changes require a system rebuild and may require a reboot to take full effect.
system-dns = DNS Configuration
system-dns-desc = Set custom DNS resolvers for your system
system-dns-servers = DNS Servers
system-dns-placeholder = 1.1.1.1, 8.8.8.8
system-dns-format = Format
system-dns-format-desc = Enter DNS servers separated by commas (e.g., 1.1.1.1, 8.8.8.8)
system-groups = User Group Membership
system-username = Username
system-username-placeholder = user
system-username-default = user
system-groups-note = Note
system-groups-note-desc = Group changes require a system rebuild. You may need to log out and back in for changes to take effect.

hardware-graphics = Graphics Drivers
hardware-graphics-desc = Configure GPU drivers for your system
hardware-gpu = Detected GPU
hardware-gpu-unknown = Unknown GPU (lspci not available)
hardware-nvidia-driver = NVIDIA Driver
hardware-nvidia-driver-desc = Select which NVIDIA driver package to use
nvidia-stable = Stable (nvidia)
nvidia-beta = Beta (nvidia-beta)
nvidia-open = Open Source (nvidia-open)
nvidia-nouveau = Nouveau (open-source, limited)
hardware-nvidia-modesetting = Modesetting
hardware-nvidia-modesetting-desc = Enable kernel modesetting (recommended for Wayland)
hardware-nvidia-power = Power Management
hardware-nvidia-power-desc = Enable experimental power management features
hardware-nvidia-open = Open Kernel Modules
hardware-nvidia-open-desc = Use open-source NVIDIA kernel modules (Turing+ GPUs)
hardware-audio = Audio
hardware-audio-desc = Configure audio server and settings
hardware-audio-server = Audio Server
hardware-audio-server-desc = Select the audio server for your system
audio-pipewire = PipeWire (recommended)
audio-pulseaudio = PulseAudio
audio-none = None
hardware-audio-lowlatency = Low Latency Audio
hardware-audio-lowlatency-desc = Enable low-latency settings for professional audio work
hardware-bluetooth = Bluetooth
hardware-bluetooth-desc = Configure Bluetooth hardware and settings
hardware-bluetooth-enable = Enable Bluetooth
hardware-bluetooth-enable-desc = Enable Bluetooth hardware and services
hardware-bluetooth-autopower = Power on at Boot
hardware-bluetooth-autopower-desc = Automatically power on Bluetooth adapter at system startup
hardware-power = Power Management
hardware-power-desc = Configure power settings for laptops and desktops
hardware-power-profile = Power Profile
hardware-power-profile-desc = Select power/performance balance
power-balanced = Balanced
power-performance = Performance
power-saver = Power Saver
hardware-tlp = TLP Power Management
hardware-tlp-desc = Advanced power management for laptops (recommended)
hardware-thermald = Thermald
hardware-thermald-desc = Thermal management daemon for Intel CPUs
hardware-thermald-arm = Intel-only - not available on ARM
hardware-note = Note
hardware-note-desc = Hardware changes require a system rebuild. Some changes may require a reboot.

network-firewall = Firewall
network-firewall-desc = Configure NixOS firewall rules
network-firewall-enable = Enable Firewall
network-firewall-enable-desc = Block incoming connections except for allowed ports
network-quick-ports = Quick Open Ports
network-quick-ports-desc = Common service ports
port-ssh = SSH (22)
port-http = HTTP (80)
port-https = HTTPS (443)
port-alt-http = Alt HTTP (8080)
network-custom-tcp = Additional TCP Ports
network-custom-tcp-placeholder = 3000, 5432, 6379
network-format = Format
network-format-desc = Enter port numbers separated by commas (e.g., 3000, 5432, 6379)
network-ssh = SSH Server
network-ssh-desc = Configure OpenSSH server for remote access
network-ssh-enable = Enable SSH Server
network-ssh-enable-desc = Allow remote SSH connections to this machine
network-ssh-port = SSH Port
network-ssh-port-desc = Port number for SSH connections
network-ssh-password = Password Authentication
network-ssh-password-desc = Allow password-based SSH login (key-only is more secure)
network-ssh-root = Root Login
network-ssh-root-desc = Control root user SSH access
root-login-no = Disabled (recommended)
root-login-prohibit = Prohibit Password (keys only)
root-login-yes = Enabled (not recommended)
network-fail2ban = Fail2Ban
network-fail2ban-desc = Ban IPs with too many failed login attempts
network-ssh-warning = Security Recommendation
network-ssh-warning-desc = Use SSH keys instead of passwords. Disable root login for better security.
network-vpn = VPN
network-tailscale = Tailscale
network-tailscale-desc = Enable Tailscale mesh VPN service
network-tailscale-after = After enabling
network-tailscale-after-desc = Run 'sudo tailscale up' to authenticate and connect to your tailnet
network-note = Note
network-note-desc = Network changes require a system rebuild to take effect.

services-hardware = Hardware Services
services-hardware-desc = Services for hardware support and drivers
services-network = Network Services
services-network-desc = Networking and connectivity services
services-remote = Remote Access
services-remote-desc = Remote desktop and access services
services-rustdesk-client = RustDesk Client
services-rustdesk-client-desc = Install rustdesk package for the client app
services-sync = Sync & Backup
services-sync-desc = File synchronization and backup services
services-desktop = Desktop Services
services-desktop-desc = Services for desktop environments
services-dev = Development
services-dev-desc = Services for software development
services-system = System
services-system-desc = Core system services
services-note = Note
services-note-desc = Service changes require a system rebuild. Some services may require additional configuration.
service-nix-option = NixOS option: { $option }

generations-available = Available Generations
generations-empty = No generations found
generations-empty-desc = This might indicate an issue with your NixOS installation
generation-n = Generation { $n }
generation-n-current = Generation { $n } (current)
generations-boot = Boot Menu
generations-boot-title = Boot into different generations
generations-boot-desc = At boot time, press a key (usually Esc or Enter) to access the GRUB/systemd-boot menu and select older generations.
generations-log = Operation Log
generations-log-placeholder = # Generation operations will be logged here

maintenance-actions = Maintenance Actions
maintenance-disk = Disk Usage
maintenance-store-size = Nix Store Size
maintenance-generations = System Generations
maintenance-generation-count = { $count } generation(s)
maintenance-disk-error = Error - see log
maintenance-log = Output Log
maintenance-log-placeholder = # Output will appear here

apply-preview = Configuration Preview
apply-preview-desc = Nix files that will be written
apply-preview-placeholder = # No changes to preview
    # Select a profile or bundles to see the configuration
apply-building = Building configuration…
apply-validating = Validating configuration…
apply-complete = ✓ Complete
apply-failed = ✗ Failed
apply-log = Build Log
apply-log-desc = Output from nixos-rebuild
apply-log-placeholder = # Output from nixos-rebuild
