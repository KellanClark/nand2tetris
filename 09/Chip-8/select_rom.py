
import sys

if len(sys.argv) < 2:
    print("Not enough arguments")
    sys.exit()

outFile = open("Rom.jack", "w")
inFile = open(sys.argv[1], "rb")

outFile.write("class Rom {\nfunction void loadRom(Array mem) {\n")

address = 512;
data = inFile.read(1)
while data:
    outFile.write("let mem[{}] = {};\n".format(address, int.from_bytes(data, byteorder='big')))
    address += 1
    data = inFile.read(1)

outFile.write("return;\n}\n}")

inFile.close()
outFile.close()
