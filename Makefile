all:
	-@rm /home/vagrant/images/test.img
	dd of=/home/vagrant/images/test.img bs=1G seek=1 count=0
