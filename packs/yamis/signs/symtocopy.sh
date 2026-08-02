#!/bin/bash

find . -depth -type l -xtype d -exec sh -c '
  for link; do
    target=$(readlink -f "$link")
    rm "$link"
    cp -rL "$target" "$link"
  done
' _ {} +
