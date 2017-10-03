#!/usr/local/bin/bash
set -e

if [ "$#" -ne 2 ] ; then
  echo "Usage: $0 <ubuntu|debian|centos|suse|fedora|scientific>  <version>" >&2
  exit 1
fi

ARCH=$(uname -m)
URL_ARCH=${ARCH};

case "${ARCH}" in
    amd64)
        ARCH=x86_64;
        ARCH_URL=x86_64;
        ;;
    *)
        echo only x86 is supported
        exit 1
        ;;
esac

if [ -x /usr/local/bin/pbzip2 ]
then
    BZIP=/usr/local/bin/pbzip2
else
    BZIP=bzip2
fi


#### End user editable vars

ROOT=zroot/jails
DISTRO=$1

if [ -z "$2" ]
then
    VSN=6
else
    VSN=$2
fi

ID=$(uuidgen)

zfs create -p ${ROOT}/$ID

>&2 echo "Prepping solitary confinement"
mkdir -p /${ROOT}/${ID}/root/jail
TARGET=/tmp/${DISTRO}-${ARCH}-${VSN}.tgz
if [ ! -f ${TARGET} ]
then
    fetch  https://download.openvz.org/template/precreated/${DISTRO}-${VSN}-${ARCH_URL}.tar.gz -o ${TARGET}
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
  "name": "${DISTRO}",
  "version": "${VSN}",
  "type": "lx-jail-dataset",
  "os": "Linux",
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
