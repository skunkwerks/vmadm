#!/bin/sh

validate_root () {
    if [ "$1" = '/' ]
    then
        echo "No no, not the global root"
        exit 1
    fi
    if echo "$1" | fgrep '..' > /dev/null
    then
        echo "invalid path: $1"
        exit 1
    fi
    if [ ! -d "$1" ]
    then
        echo "relative path not allowed: $1"
        exit 1
    fi
    if [ ! -d "$1/root" ]
    then
        echo "No root directory: $1/root"
        exit 1
    fi
    if [ ! -d "$1/root/jail" ]
    then
        echo "No no jail: $1/root/jail"
        exit 1
    fi
    if ! zfs list "$1" > /dev/null
    then
        echo "Not a ZFS dataset"
        exit 1
    fi
}

install_brand_files() {
    brand_root="$1"
    jail_root="$2"

    brands_src=$(dirname ${brand_root})
    brands_target="${jail_root}/root/${brands_src}"

    # delete the old brand
    rm -r ${brands_target}

    # create a new folder for the brand
    mkdir -p ${brands_target}

    # copy over our brand
    cp -r ${brand_root} ${brands_target}
    cp -r ${brands_src}/shared ${brands_target}

}


## Find files that do not beling in the jail root, which is everything but
## jail, the rest will be populated by us
clean_outer_root() {
    jail_root=$1
    validate_root "${jail_root}"
    find "${jail_root}/root" \
         -not -path "${jail_root}/root/config" \
         -not -path "${jail_root}/root/config/*" \
         -not -path "${jail_root}/root/jail" \
         -not -path "${jail_root}/root/jail/*" \
         -not -path "${jail_root}/root" \
         -delete

}
