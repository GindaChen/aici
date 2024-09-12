#!/bin/bash

# Define the directory and base filename
DIR="./"
BASE_FILE="controller_simple.py"

# Define the values for per_token_control
VALUES=(1 2 4 8 16 32 64 128)

# Loop through each value and create a new file
for VALUE in "${VALUES[@]}"; do
    # Create a new filename
    NEW_FILE="controller_simple_${VALUE}.py"
    
    # Copy the original file to the new file
    cp "$DIR/$BASE_FILE" "$DIR/$NEW_FILE"
    
    # Update the per_token_control value in the new file
    sed -i "s/per_token_control = .*/per_token_control = $VALUE/" "$DIR/$NEW_FILE"
    
    echo "Created $NEW_FILE with per_token_control set to $VALUE"
done