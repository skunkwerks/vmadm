index.json: manifests/*.json
	jq -sM 'sort_by(.version)' manifests/*.json > index.json
