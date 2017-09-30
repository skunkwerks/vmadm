Private image repositories can be set up with a few steps.

1) a s3 bucket
2) configure the bucket in `../s3.mk`
3) put the json manifests in `./manifests`
4) put the datasets in `./files`
5) run `gmake`
6) set public read permissions on the bucket
7) configure cloud flare redirects:

![Cloudflare Config](https://imgur.com/X4rI04Y.png "Cloudflare Config")


PS: there are sure other options, YMMV
