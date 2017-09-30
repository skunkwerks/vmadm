include ../s3.mk

sync: index.json
	aws s3 sync --exclude '*.DS_Store' --exclude '*.gitignore' --exclude 'Makefile' . $(BUCKET)
index.json: manifests/*.json
	jq -sM 'sort_by(.version)' manifests/*.json > index.json
