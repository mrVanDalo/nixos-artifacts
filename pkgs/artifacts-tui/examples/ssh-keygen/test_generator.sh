#!/usr/bin/env bash
export PATH=/nix/store/c9k1j0y8p3wx2yd48zrmb7r4il3c0h2z-openssh-10.0p2/bin/:$PATH

ssh-keygen -t ed25519 -N "" -f $out/id_ed25519
echo "$machine ssh key for ${hostname} = $(cat $out/id_ed25519.pub)"