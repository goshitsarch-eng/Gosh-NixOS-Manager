# Science & Math Bundle
# Managed by NixOS Toolkit
#
# Scientific computing, math tools, and LaTeX.

{ config, lib, pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    # Mathematical computing
    octave
    julia-bin
    R
    rstudio

    # Python scientific stack
    (python3.withPackages (ps: with ps; [
      numpy
      scipy
      matplotlib
      pandas
      jupyter
      sympy
    ]))

    # Plotting
    gnuplot

    # LaTeX
    texlive.combined.scheme-full
    texstudio

    # Other math tools
    maxima
    wxmaxima
    geogebra

    # Data analysis
    pspp  # SPSS alternative
  ];
}
