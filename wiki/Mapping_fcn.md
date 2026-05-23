## **Mapping a function to a gate diagram**

To create a gate diagram for a function entered by truth table or logic equation:

1.  Select **Operation \| Map to Gates**.
2.  In the dialog that appears, select the gates you want to use. Note: Your selections must include a NAND or a NOR gate type.
3.  Choose whether you want to optimize for standard logic ICs or for die area. If you optimize for standard ICs the program will try to minimize the total number of IC packages. If you optimize for die area the program will pick gates to minimize the die area that would be required in a custom IC.
4.  Click OK to submit your selections. When the mapping is complete the gate diagram will appear.

**Notes:**\

- Any function can be mapped to a gate diagram using only NOR gates or NAND gates.
- When you map a function to a gate diagram, it will be minimized automatically if it is not already minimized.
- When you optimize for standard logic ICs, Logic Friday may substitute NAND or NOR gates for inverters. For example, if the function requires three NAND gates and one inverter, the program will substitute a NAND for the inverter, saving an IC package. When a NAND gate is used as an inverter, only one of its inputs is used and the remaining inputs are committed to a constant logic 1. Similarly, when a NOR gate is used as an inverter, the unused inputs are committed to a constant logic 0.

See also: [Gate diagram limitations](Limitations.md).
