# Fonts Collection Bundle
# Managed by NixOS Toolkit
#
# Popular fonts for development, documents, and design.

{ config, lib, pkgs, ... }:

{
  fonts = {
    packages = with pkgs; [
      # Nerd Fonts (programming fonts with icons)
      nerd-fonts.fira-code
      nerd-fonts.jetbrains-mono
      nerd-fonts.hack
      nerd-fonts.droid-sans-mono
      nerd-fonts.ubuntu-mono

      # Sans-serif
      inter
      roboto
      open-sans
      source-sans
      ubuntu_font_family

      # Serif
      source-serif
      libertine
      eb-garamond

      # Monospace (programming)
      fira-code
      jetbrains-mono
      source-code-pro
      cascadia-code
      iosevka

      # Google fonts
      noto-fonts
      noto-fonts-cjk-sans
      noto-fonts-emoji

      # Microsoft fonts (for compatibility)
      corefonts
      vistafonts

      # Icons
      font-awesome
      material-design-icons
    ];

    fontconfig = {
      enable = true;
      defaultFonts = {
        serif = [ "Noto Serif" "DejaVu Serif" ];
        sansSerif = [ "Inter" "Noto Sans" "DejaVu Sans" ];
        monospace = [ "JetBrains Mono" "Fira Code" "DejaVu Sans Mono" ];
        emoji = [ "Noto Color Emoji" ];
      };
    };
  };
}
