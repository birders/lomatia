# vi: set ft=ruby :

Vagrant.configure("2") do |config|
  config.vm.hostname = "lomatia-dev"
  config.vm.box = "generic/arch"

  config.vm.provider "virtualbox" do |vb|
    vb.memory = "1024"
  end

  config.vm.provision "shell", privileged: false, inline:
   %[curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly -y &> /dev/null]

  config.vm.provision "shell", privileged: true, inline:
    %[yes | pacman -S tmux neovim git ripgrep htop &> /dev/null]
end
