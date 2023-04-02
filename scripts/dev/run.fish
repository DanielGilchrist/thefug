#!/bin/fish

set command (cargo run)

if "$command" = "No fugs given."
  echo "$command"
else
  echo "Running: $command"
  eval "$command"
end
