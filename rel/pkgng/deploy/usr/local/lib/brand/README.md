While there are technically no brands for jails, vmadm borrows from illumos when it comes to the concept of brands.

The basic concept is that a jail can have a brand that defines how it is installed, started, stopped, logged in and so on. Having a brand allows decoupling this from both the jail (dataset) as well as form the compiled part of vmadm.

The brand is also what operates the outer jail, it's part of the brands job to ensure this is not containing any problematic files. The current approach of the brands is to clean it out when the jail is created and fill it with relevant files.



Current brands are:

* jail - a classical FreeBSD jail
* lx-jail - a jail running a linux systems supporting
    * redhat (centos etc)
    * debian
    * ubuntu

Brands live in `/usr/local/lib/brand` on the system and `rel/pkgng/deploy/usr/local/lib/brand/` as part of the code. There is a 'special' brand named `shared` which itself is less of a brand then rather a collection of shared code (thanks Solaris for that idea!).

The minimal requirement for a brand is the brand config file named `config.toml`. Inside the config file there is 1 required key `modname` which should match the name of the brand, and a few sections.

```
[<section>]
cmd="./acommand"
args=["some", "argument"]
```


Each section describes one step in the lifetime of a branded jail, currently the following lifetime steps exist:

* **install** - executed once right after the jail zfs is created (for example to clean up the outer jail or install required binaries)
* **init** - called on the host before the jail is booted (for example for mounting the devfs)
* **boot** - called **inside the outer jail** to boot the inner jail (this should execute a `jail -c` call)
* **halt** - called to halt the inner jail
* **halted** - called after the outer jail was stopped (for cleanup/unmounting)
* **login** - called when `vmadm login` is used the resulted call will be the 'shell' provded

This calls and the arguments can take placeholders:

* `{inner_id}` - the jid of the inner jail (only halt, login)
* `{ounter_id}` - the jid of the outer jail (only halt, login)
* `{jail_uuid}` - uuid if the jail
* `{jail_root}` - root of the jail
* `{brand_root}` - root of this brand
* `{hostname}` - hostname of the jail

An example can be found in the `jail` or `lx-jail` folders.

There are two well defined directories in a jail:

- `/jail` the root jail
- `/config`

The `/config` directoy might hold the following files:

- `/config/resolvers` configured resolvers one resolver per line
- `/config/root_authorized_keys` - authorized ssh keys for the root user
- `/config/user_script` - a script the user has asked to be executed on boot
