#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname $0)"

tmux split -v 'source ~/.nvm/nvm.sh; nvm use stable; cd www; npm run start'
tmux resize-pane -t 2 -y 5
./watch_build.sh
