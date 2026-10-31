#!/bin/zsh
# Render the review set after building with GLINT_ASSETS pointing at the live art.
python3 "${0:A:h}/review.py" "${1:-out/review/glasswater}" "${@:2}"
