While there are technically no brands for jails, vmadm borrows from illumos when it comes to the concept of brands.

The basic concept is that a jail can have a brand that defines how it is installed, started, stopped, logged in and so on. Having a brand allows decoupling this from both the jail (dataset) as well as form the compiled part of vmadm.

The brand is also what operates the outer jail, it's part of the brands job to ensure this is not containing any problematic files. The current approach of the brands is to clean it out when the jail is created and fill it with relevant files.



Current brands are:

* jail - a classical FreeBSD jail
* lx-jail - a jail running a linux systems supporting
    * redhat (centos etc)
    * debian
    * ubuntu