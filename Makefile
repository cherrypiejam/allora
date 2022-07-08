all:
	-@rm test.img
	dd of=test.img bs=1G seek=1 count=0
