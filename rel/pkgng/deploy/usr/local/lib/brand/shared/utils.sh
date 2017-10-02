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
