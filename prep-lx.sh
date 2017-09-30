#!/usr/local/bin/bash
#set -x
declare -a DIRS=("bin" "dev" "mnt" "proc" "tmp" "etc/defaults")
declare -a EXECS=("COPYRIGHT" "/libexec/ld-elf.so.1" "bin/sh" "/sbin/ifconfig" "/sbin/route" "usr/sbin/jail")


ARCH=$(uname -m)
URL_ARCH=${ARCH};

case "${ARCH}" in
    amd64)
        ARCH=x86_64;
        ARCH_URL=x86;
        ;;
    arm64)
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

if [ -z "$1" ]
then
    ROOT=zroot/jails
else
    ROOT=$1
fi

if [ -z "$2" ]
then
    VSN=6
else
    VSN=$2
fi

ID=$(uuidgen)

zfs create -p ${ROOT}/$ID

>&2 echo "Prepping outside jail..."

declare -a FILES

for d in "${DIRS[@]}"
do
    mkdir -p /${ROOT}/$ID/root/$d
    chown root:wheel /${ROOT}/$ID/root/$d
    chmod 775 /${ROOT}/$ID/root/$d
done

cp /etc/defaults/devfs.rules /${ROOT}/$ID/root/etc/defaults

for e in "${EXECS[@]}"
do
    FILES=("${FILES[@]}" $(ldd -a /$e 2> /dev/null | awk '/=>/{print $(NF-1)}'))
    FILES=("${FILES[@]}" "$e")
done

for f in "${FILES[@]}"
do
    mkdir -p /${ROOT}/$ID/root/$(dirname $f)
    cp /$f /${ROOT}/$ID/root/$f
done

>&2 echo "Prepping solitary confinement"
mkdir -p /${ROOT}/${ID}/root/jail
TARGET=/tmp/centos-${ARCH}-${VSN}.tgz
if [ ! -f ${TARGET} ]
then
    fetch  https://download.openvz.org/template/precreated/centos-${VNS}-${ARCH_URL}.tar.gz -o ${TARGET}
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
  "name": "CentOS",
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

>&2 echo "Jail is ready. Snapshot if needed"
echo $ID
