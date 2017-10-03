#!/bin/sh


# The following code is taken from SmartOS:
# https://github.com/joyent/illumos-joyent/blob/914d7a738527f83b1e811b066563842839f125a7/usr/src/lib/brand/lx/zone/lx_boot.ksh

#
# CDDL HEADER START
#
# The contents of this file are subject to the terms of the
# Common Development and Distribution License (the "License").
# You may not use this file except in compliance with the License.
#
# You can obtain a copy of the license at usr/src/OPENSOLARIS.LICENSE
# or http://www.opensolaris.org/os/licensing.
# See the License for the specific language governing permissions
# and limitations under the License.
#
# When distributing Covered Code, include this CDDL HEADER in each
# file and include the License file at usr/src/OPENSOLARIS.LICENSE.
# If applicable, add the following below this CDDL HEADER, with the
# fields enclosed by brackets "[]" replaced with your own identifying
# information: Portions Copyright [yyyy] [name of copyright owner]
#
# CDDL HEADER END
#
#
# Copyright (c) 2009, 2010, Oracle and/or its affiliates. All rights reserved.
# Copyright 2015, Joyent, Inc.
# Copyright 2017 ASS-Einrichtungssysteme GmbH, Inc.
#
# lx boot script.
#
# The arguments to this script are the zone name and the zonepath.
#


detect_distro() {
    jail_root="$1"
    if [ -f ${jail_root}/etc/redhat-release ]; then
	      echo "redhat"
    elif [ -f ${jail_root}/etc/lsb-release ]; then
	      if fgrep -s Ubuntu ${jail_root}/etc/lsb-release > /dev/null; then
		        echo "ubuntu"
	      elif [ -f ${jail_root}/etc/debian_version ]; then
		        echo "debian"
	      fi
    elif [ -f ${jail_root}/etc/debian_version ]; then
	      echo "debian"
    elif [ -f ${jail_root}/etc/alpine-release ]; then
	      distro="busybox"
    elif [ -f ${jail_root}/etc/SuSE-release ]; then
	      distro="suse"
    fi
    exit 3
}
