#!/bin/sh
# Makes the pictures of the website. Run it from the repository root.
# The examples use spec/icons, so they match the golden files.
set -eu
for n in arch-checkout plan-ratelimit seq-login journey-checkout erd-schemas state-order; do
  swe-diagrams render "spec/fixtures/$n.sd" -o "site/img/$n.svg" --animate --offline --icons spec/icons
done
swe-diagrams render site/diagrams/stack.sd -o site/img/stack.svg --animate
