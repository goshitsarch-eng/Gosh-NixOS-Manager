# Multimedia Bundle
# Managed by NixOS Toolkit
#
# This bundle provides multimedia applications for audio, video, and graphics.

{ config, lib, pkgs, ... }:

{
  # Enable PipeWire for audio
  security.rtkit.enable = true;
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
    jack.enable = true;
  };

  # Multimedia packages
  environment.systemPackages = with pkgs; [
    # Video players
    vlc
    mpv
    celluloid          # GTK frontend for mpv

    # Audio
    audacity
    pavucontrol        # PulseAudio volume control
    easyeffects        # Audio effects for PipeWire

    # Graphics
    gimp
    inkscape
    krita

    # Screen recording
    obs-studio
    peek               # GIF recorder

    # Media info
    mediainfo
    ffmpeg

    # Codecs
    gst_all_1.gstreamer
    gst_all_1.gst-plugins-base
    gst_all_1.gst-plugins-good
    gst_all_1.gst-plugins-bad
    gst_all_1.gst-plugins-ugly
  ];
}
