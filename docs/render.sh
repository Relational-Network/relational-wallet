#------------------------------------------------------------------------------
# Relational Wallet SDK
# Copyright (C) 2025  Relational

# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.

# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#------------------------------------------------------------------------------

#!/usr/bin/env bash
set -e

INPUT_DIR="sequence"
OUTPUT_DIR="seq-diagrams"

# Create output directory if missing
mkdir -p "$OUTPUT_DIR"

echo "Rendering all .puml files in '$INPUT_DIR'..."

for file in $INPUT_DIR/*.puml; do
    filename=$(basename "$file" .puml)
    plantuml -tpng "$file" -o "../$OUTPUT_DIR"
    # plantuml -tsvg "$file" -o "../$OUTPUT_DIR"
    echo "Rendered: $filename"
done

echo "Done! Files available in '$OUTPUT_DIR/'."