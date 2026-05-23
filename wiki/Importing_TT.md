## **Importing a truth table**

You can import a truth table for a function as a CSV file. Most spreadsheet applications support this file type.

The file must be formatted as follows:\

1.  Any lines preceding the truth table must be blank or begin with the character '%' (comment).
2.  The first line of the truth table must be the variable names in the form \<input variable cells\>\<empty cell\>\<output variable cells\>. For variable naming rules, see [Equation Syntax](Syntax.md).
3.  The truth table value lines must immediately follow the variable names line, and must be in the form \<input value cells\>\<empty cell\>\<output value cells\>. Valid values are '0', '1', and 'X'.
4.  The truth table ends with a blank line, a comment, or end-of-file.

You can use don't-care ('X') both for input values and for output values. Input don't-cares are for editing convenience and have no effect on minimization results. On import, they are expanded to create a complete truth table. Any known output don't-cares should be included, as they may improve minimization results.

You do not have to create a complete table. If a term is not found in the table, all outputs for that term will be set to 0.

Example:\
` %Truth Table`\
`A,B,C,,F0,F1`\
`0,0,0,,1,1`\
`0,1,X,,0,1`\
`1,X,X,,1,1`\

In this example, the term 0,0,1 was not specified, so the outputs for it will be set to 0.

To import a truth table as a CSV file:

1.  Select **File \| Import Truth Table...**. The "Open" dialog appears.
2.  Select the folder and the file. Note that the file does not have to have the extension .csv.
3.  Click **Open** and the file will be imported. If any errors are found, a message will appear.
