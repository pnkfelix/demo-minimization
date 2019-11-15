#!/usr/bin/env bash

# Builds all of the board documentation into doc/rustdoc.
# The list of boards to build documentation for is generated from
# list_boards.sh.

set -e

# Parse a search-index.js file to get the known crates.
function get_known_crates {
	FILE=$1
	FOUND_CRATES=$(grep -o 'searchIndex\["[a-zA-Z0-9_-]*"\]' $FILE | cut -d'"' -f2)
	echo $FOUND_CRATES
}

# Function to add new board.
function add_board {
	BOARD=$1

	echo "Building docs for $BOARD"
	pushd boards/$BOARD > /dev/null
	make doc
	TARGET=`make -s show-target`
	popd > /dev/null

	EXISTING_CRATES=$(get_known_crates doc/rustdoc/search-index.js)
	BUILT_CRATES=$(get_known_crates boards/$BOARD/target/$TARGET/doc/search-index.js)

	# Get any new crates.
	NEW_CRATES=" ${BUILT_CRATES[*]} "
	for item in ${EXISTING_CRATES[@]}; do
		NEW_CRATES=${NEW_CRATES/ ${item} / }
	done

	# Copy those crates over.
	for item in ${NEW_CRATES[@]}; do
		cp -r boards/$BOARD/target/$TARGET/doc/$item doc/rustdoc/

		# Add the line to the search-index.js file.
		grep "searchIndex\[\"$item\"\]" boards/$BOARD/target/$TARGET/doc/search-index.js >> doc/rustdoc/search-index.js

		# Then need to move `initSearch(searchIndex);` to the bottom.
		#
		# Nothing in-place (i.e. `sed -i`) is safely cross-platform, so
		# just use a temporary file.
		#
		# First remove it.
		grep -v 'initSearch(searchIndex);' doc/rustdoc/search-index.js > doc/rustdoc/search-index-temp.js
		# Then add it again.
		echo "initSearch(searchIndex);" >> doc/rustdoc/search-index-temp.js
		mv doc/rustdoc/search-index-temp.js doc/rustdoc/search-index.js
	done
}

function build_all_docs {
    # Need to build one board to get things started.
    BOARD=$1
    shift
    echo "Building docs for $BOARD"
    pushd boards/$BOARD > /dev/null
    make doc
    TARGET=`make -s show-target`
    popd > /dev/null
    cp -r boards/$BOARD/target/$TARGET/doc doc/rustdoc
    ## Now can do all the rest.
    for BOARD in $*
    do
        echo "Now building for $BOARD"
        add_board $BOARD
    done
}

# Delete any old docs
rm -rf doc/rustdoc

# Get a list of all boards
ALL_BOARDS=$(./tools/list_boards.sh)
# Build documentation for all of them
build_all_docs $ALL_BOARDS

# Temporary redirect rule
# https://www.netlify.com/docs/redirects/
cat > doc/rustdoc/_redirects << EOF
# While we don't have a home page :/
/            /kernel            302
EOF
