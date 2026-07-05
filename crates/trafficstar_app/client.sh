cargo run -- -t Client -a 81.231.235.221 5201 \
 --interfaces /home/root/saved/trafficstar/interfaces \
 --test-parameters /home/root/saved/trafficstar/test_sessions/ \
 --tor-parameters /home/root/saved/trafficstar/tor/ \
 --mullvad-parameters /home/root/saved/trafficstar/mullvad/ \
 --mullvad-accounts $TRAFFICSTAR_MULLVAD_ACCOUNT\
 --storage /home/root/saved/\
 --run-tests $1\
 --directory-prefix test-runs