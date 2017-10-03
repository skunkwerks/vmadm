#!/usr/local/bin/bash
set -e

ARCH=$(uname -m)
URL_ARCH=${ARCH};

case "${ARCH}" in
    amd64)
        ARCH=x86_64;
        ;;
    arm64)
        URL_ARCH=arm64/aarch64
        ;;
esac

if [ -x /usr/local/bin/pbzip2 ]
then
    BZIP=/usr/local/bin/pbzip2
else
    BZIP=bzip2
fi


#### End user editable vars

if [ -z "$1" ]
then
    ROOT=zroot/jails
else
    ROOT=$1
fi

if [ -z "$2" ]
then
    VSN=`uname -r`
else
    VSN=$2
fi

ID=$(uuidgen)

zfs create -p ${ROOT}/$ID

>&2 echo "Prepping outside jail..."


>&2 echo "Prepping solitary confinement"
mkdir -p /${ROOT}/${ID}/root/jail
TARGET=/tmp/base-${ARCH}-${VSN}.tgz
if [ ! -f ${TARGET} ]
then
    fetch ftp://ftp.freebsd.org/pub/FreeBSD/releases/${URL_ARCH}/${VSN}/base.txz -o ${TARGET}
else
    echo "Image seems to already exist, not re-downloading, delete ${TARGET} to force re-download"
fi

tar -xf ${TARGET} -C /${ROOT}/${ID}/root/jail/

zfs snapshot ${ROOT}/${ID}@final

zfs send ${ROOT}/${ID}@final | ${BZIP} > ${ID}.dataset

SIZE=`ls -l ${ID}.dataset | cut -f 5 -w`
SHA=`sha1 -q ${ID}.dataset`
DATE=`date -u "+%Y-%m-%dT%H:%M:%SZ"`
cat <<EOF > $ID.json
{
  "v": 2,
  "uuid": "${ID}",
  "name": "FreeBSD",
  "version": "${VSN}",
  "type": "jail-dataset",
  "os": "freebsd",
  "files": [
    {
      "size": ${SIZE},
      "compression": "bzip2",
      "sha1": "${SHA}"
    }
  ],
  "requirements": {
    "architecture": "${ARCH}",
    "networks": [{"name": "net0", "description": "public"}]
  },
  "published_at": "${DATE}",
  "public": true,
  "state": "active",
  "disabled": false
}
EOF

IMG_FILE=/var/imgadm/images/$(echo $ROOT | sed 's/\//-/g')-$ID.json

echo  "{\"zpool\":\"${ROOT}\", \"manifest\":" > $IMG_FILE
cat $ID.json >> $IMG_FILE
echo "}" >> $IMG_FILE


>&2 echo "Jail is ready. Snapshot if needed"
echo $ID
