/* 00411f69 FUN_00411f69 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void FUN_00411f69(HWND param_1,uint param_2)

{
  uint unaff_retaddr;
  char local_10c [260];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  switch(param_2 >> 0x10) {
  case 1:
    FUN_0043ed39(local_10c,
                 (byte *)
                 "Invalid character in the truth table at line %d.\nAll truthtable cells must be \'0\', \'1\', or \'X\'."
                );
    MessageBoxA(param_1,local_10c,"Import Error",0);
    break;
  case 2:
    FUN_0043ed39(local_10c,
                 (byte *)
                 "Variable names were not found. Names for inputs and outputs\nmust be in the line immediately preceding the truth table."
                );
    MessageBoxA(param_1,local_10c,"Import Error",0);
    break;
  case 3:
    FUN_0043ed39(local_10c,
                 (byte *)
                 "An input or output variable name could not be used because\nit is too long. Names must be no more than %d characters in length."
                );
    MessageBoxA(param_1,local_10c,"Import Error",0);
    break;
  case 4:
    FUN_0043ed39(local_10c,
                 (byte *)
                 "The count of variable names does not equal the count\nof variables, or the count of output names does not\nequal the count of outputs."
                );
    MessageBoxA(param_1,local_10c,"Import Error",0);
    break;
  case 5:
    FUN_0043ed39(local_10c,(byte *)"The name of an input or output is used more than once.");
    MessageBoxA(param_1,local_10c,"Import Error",0);
    break;
  case 6:
    FUN_0043ed39(local_10c,(byte *)"Invalid character in a variable name: \'%c\'");
    MessageBoxA(param_1,local_10c,"Import Error",0);
    break;
  case 7:
    FUN_0043ed39(local_10c,(byte *)"No truth table found, or format error.");
    MessageBoxA(param_1,local_10c,"Import Error",0);
    break;
  case 8:
    FUN_0043ed39(local_10c,(byte *)"The truth table must have 2 to %d inputs and 1 to %d outputs.");
    MessageBoxA(param_1,local_10c,"Import Error",0);
    break;
  case 9:
    FUN_0043ed39(local_10c,(byte *)"Variable name declaration not found where expected.");
    MessageBoxA(param_1,local_10c,"Import Error",0);
    break;
  case 10:
    FUN_0043ed39(local_10c,
                 (byte *)
                 "Conflict assigning an output value at line %d. A different value was assigned previously."
                );
    MessageBoxA(param_1,local_10c,"Import Error",0);
    break;
  default:
    FUN_0043ed39(local_10c,(byte *)"The operation could not be completed.");
    MessageBoxA(param_1,local_10c,"Import Error",0);
  }
  return;
}
