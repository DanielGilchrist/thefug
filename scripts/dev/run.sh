#!/bin/bash

command=$(cargo run)

if [ "$command" = "No fugs given." ]; then
  echo "$command"
else
  echo "Running: $command"
  eval "$command"
fi
