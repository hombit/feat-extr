These files were used for feature extraction in ZTF Data Release 23. We ran `run_dr23.sh` on cygnus (Sternberg Astronomical Institute machine) with `./run_dr23.sh 100 r snad_clf sai.db.ztf.snad.space` command (it took 235 hours on a 14 cores of dual Intel Xeon Gold 5118). 

In `docker-compose.yml` you may see particular user setups, which were used during extraction. (Note that if you plan to run the code on behalf of your user (not the root), then you need to make
the appropriate edits to this file and create a directory in advance in which the program output will be saved. Otherwise, you will get a permission rights error)

If you want to create your own feature sample, then you need to describe it in `src/features.rs`. (If you have changed `src/features.rs` after building containers, you will need to rebuild them again)

If you need to extract features on LPC server, then you may check `run_dr17.sh` file. Also, you will need to change `docker-compose.yml` (uncomment last rows).
