## **Minimizing a function**

The **Minimize** operation uses *espresso* to minimize the number of product terms and the number of literals in a function. The **Fast** option produces a result that is good, but may not be truly minimal. The **Exact** option produces a result that is guaranteed to have a minimal number of product terms, but the solution may take a significant amount of time for functions with more than eight inputs.

If the problem has multiple outputs, you can choose either **Independent** or **Joint** minimization. The **Joint** option minimizes the total number of product terms, but may not strictly minimize each output. The **Independent** option strictly minimizes each output, but may result in more product terms. Hint: If you are mapping the problem to gates, choose **Independent** minimization, as that option will usually result in a simpler gate diagram.

To minimize a function:

1.  Select **Operation \| Minimize**.
2.  In the dialog box, choose **Fast** or **Exact**. If the problem has multiple outputs, choose **Independent** or **Joint** minimization.
3.  Select OK. *Espresso* will run and the results will be displayed in the Truth Table window and the Logic Equation window.

**Note:**

- When you map a function to a gate diagram, it will be minimized automatically if it is not already minimized.
