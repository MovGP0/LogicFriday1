## **Creating a C code lookup function**

If you have a logic function that you need to implement in software or firmware you can create a fast, compact C code lookup function for it.

The lookup function is generated as a stand-alone module that can be added to your source files. You call the function with arguments representing the input variable values, and the function returns 1 or 0.

To generate a C code lookup function:

1.  Select **Operation \| Generate C Lookup Function...**.
2.  In the dialog that appears, select the size of type *unsigned int* on the target processor and compiler. This information is needed to properly distribute the output truth values over the bits of a data table.
3.  Choose whether you want the function to have parameters for each individual input variable, or a single parameter that will be the truth table row number (minterm). For example, suppose the logic function has output F and two input variables, A and B. If you choose individual parameters the lookup function header will be\
        `int lu_F( int A, int B )`\
    If you choose the single parameter the header will be\
        `int lu_F( unsigned int nTerm )`\
    The parameter nTerm can take arguments of 0, 1, 2, and 3.
4.  Click **OK** and the "Save As" dialog will appear. After you save the file the new function will be displayed in a Notepad window.

**Notes:**

- If the problem contains multiple outputs, an independent lookup function will be generated for each output.
- If the logic function contains "don't care" outputs for any rows, they are converted to logic 1, or true.
- Lookup functions do not include any error checking, and in general may need editing to compile on the target compiler.
- You should always verify that the lookup function works correctly on the target system.
