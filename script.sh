
function get_system(){
    echo "Getting system information..."
    echo "OS Version: $(uname -s) $(uname -r)"
    echo "Hostname: $(hostname)"
    echo "Current user: $(whoami)"
    echo "System uptime: $(uptime)" 
    echo "foo== {{ foo }}"
}

echo "global script"

function go(){
    echo "go function in global script"
}
set -e

get_system

sleep 2
