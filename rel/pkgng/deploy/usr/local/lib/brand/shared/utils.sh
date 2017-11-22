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

install_etc_resolv_conf() {
    jail_root=$1
    if [ -f "${jail_root}/root/config/resolvers" ]
    then
        for r in $(cat "${jail_root}/root/config/resolvers")
        do
            echo "nameserver ${r}" >> ${jail_root}/root/jail/etc/resolv.conf
        done
    fi
}
install_authorized_keys() {
    jail_root=$1
    if [ -f "${jail_root}/root/config/root_authorized_keys" ]
    then
        mkdir -p "${jail_root}/root/jail/root/.ssh"
        cp "${jail_root}/root/config/root_authorized_keys" "${jail_root}/root/jail/root/.ssh/authorized_keys"
    fi

}

expand_linked() {
    for file in $1
    do
        echo "${file}"
        ldd -a "${file}" 2> /dev/null | awk '/=>/{print $(NF-1)}'
    done
}

read_routes() {
    # we run the interface routes first
    while read route gw
    do
        if ifconfig "${gw}" 2> /dev/null
        then
            /sbin/route add "${route}" -iface "${gw}"
            echo "route: $route"
            echo "gw: $gw"
        fi
    done < "/config/routes"

    # now we use network routes
    while read route gw
    do
        if ! ifconfig "${gw}" 2> /dev/null
        then
            /sbin/route add "${route}" "${gw}"
        fi

    done < "/config/routes"
}
