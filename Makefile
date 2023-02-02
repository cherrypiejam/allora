all:
	-@rm test.img
	dd of=test.img bs=500M seek=1 count=0
