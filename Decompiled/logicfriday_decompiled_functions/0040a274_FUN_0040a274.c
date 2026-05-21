/* 0040a274 FUN_0040a274 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void __cdecl FUN_0040a274(HWND param_1,uint param_2)

{
  uint unaff_retaddr;
  char local_10c [260];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  switch(param_2 >> 0x10) {
  case 1:
    FUN_0043ed39(local_10c,(byte *)"Illegal character: \'%c\' (ASCII %d)");
    MessageBoxA(param_1,local_10c,"Syntax Error",0);
    break;
  case 2:
    FUN_0043ed39(local_10c,
                 (byte *)
                 "Too many variables. This version of Logic Friday is limited to %d variables.");
    MessageBoxA(param_1,local_10c," Limit Error",0);
    break;
  case 3:
    FUN_0043ed39(local_10c,(byte *)"Syntax error in equation.");
    MessageBoxA(param_1,local_10c,"Syntax Error",0);
    break;
  case 4:
    FUN_0043ed39(local_10c,
                 (byte *)
                 "There is not enough memory to complete the operation,To free up available memory, close programs, projects,or windows you aren\'t using, and then try again. Error code %d"
                );
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 5:
    FUN_0043ed39(local_10c,(byte *)"No function is selected.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 6:
    FUN_0043ed39(local_10c,(byte *)"Repeated variable in a product term: \'%c\'");
    MessageBoxA(param_1,local_10c,"Syntax Error",0);
    break;
  case 7:
    FUN_0043ed39(local_10c,(byte *)"Could not create function window.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  default:
    FUN_0043ed39(local_10c,(byte *)"Unidentified error.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 10:
    FUN_0043ed39(local_10c,(byte *)"Could not create edit window.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0xb:
    FUN_0043ed39(local_10c,(byte *)"Verify failed: true term %d not covered");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0xc:
    FUN_0043ed39(local_10c,(byte *)"Verify failed: false term %d is covered");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0xd:
    FUN_0043ed39(local_10c,(byte *)"Verify failed: false term %d marked essential");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0xe:
    FUN_0043ed39(local_10c,(byte *)"Could not create table windows.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0xf:
    FUN_0043ed39(local_10c,(byte *)"Error in dwTrueCnt.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x10:
    FUN_0043ed39(local_10c,(byte *)"Display table verify failed at term %d.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x11:
    FUN_0043ed39(local_10c,(byte *)"Flag error clearing the input truthtable.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x12:
    FUN_0043ed39(local_10c,(byte *)"Unmatched \'%c\' in input.");
    MessageBoxA(param_1,local_10c,"Syntax Error",0);
    break;
  case 0x13:
    FUN_0043ed39(local_10c,
                 (byte *)
                 "Variable names may have only letters, digits, periods, underscores, and brackets.\n The name must begin with a letter or underscore. Illegal char: \'%c\' (ASCII %d)"
                );
    MessageBoxA(param_1,local_10c,"Syntax Error",0);
    break;
  case 0x14:
    FUN_0043ed39(local_10c,(byte *)"Variable names are limited to %d characters.");
    MessageBoxA(param_1,local_10c,"Syntax Error",0);
    break;
  case 0x15:
    FUN_0043ed39(local_10c,(byte *)"Syntax error: missing \'%c\'");
    MessageBoxA(param_1,local_10c,"Syntax Error",0);
    break;
  case 0x16:
    FUN_0043ed39(local_10c,(byte *)"Function name is undefined.");
    MessageBoxA(param_1,local_10c,"Syntax Error",0);
    break;
  case 0x17:
    FUN_0043ed39(local_10c,(byte *)"Unrecognized command or function.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x18:
    FUN_0043ed39(local_10c,
                 (byte *)"An output variable may not appear on the right hand side of an equation.")
    ;
    MessageBoxA(param_1,local_10c,"Syntax Error",0);
    break;
  case 0x19:
    FUN_0043ed39(local_10c,(byte *)"Could not open the log file.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x1a:
    FUN_0043ed39(local_10c,(byte *)"Unable to map the function.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x1b:
    FUN_0043ed39(local_10c,(byte *)"Error creating minimize thread.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x1c:
    FUN_0043ed39(local_10c,(byte *)"Espresso error %d.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x1d:
    FUN_0043ed39(local_10c,(byte *)"File error %d.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x1e:
    FUN_0043ed39(local_10c,(byte *)"You must assign an output variable with \'=\'.");
    MessageBoxA(param_1,local_10c,"Syntax Error",0);
    break;
  case 0x1f:
    FUN_0043ed39(local_10c,(byte *)"Illegal character: \'%c\' (ASCII %d)");
    MessageBoxA(param_1,local_10c,"Syntax Error",0);
    break;
  case 0x20:
    FUN_0043ed39(local_10c,(byte *)"Could not attach console.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x21:
    FUN_0043ed39(local_10c,(byte *)"Gates verification failed: false term %d yields true");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x22:
    FUN_0043ed39(local_10c,(byte *)"Gates verification failed: true term %d yields false");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x23:
    FUN_0043ed39(local_10c,(byte *)"Gate translation error %d.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x24:
    FUN_0043ed39(local_10c,(byte *)"misII error %d.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x25:
    FUN_0043ed39(local_10c,(byte *)"File does not appear to be the correct type.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x26:
    FUN_0043ed39(local_10c,(byte *)"Page Setup failed.");
    MessageBoxA(param_1,local_10c,"Setup Error",0);
    break;
  case 0x27:
    FUN_0043ed39(local_10c,(byte *)"An error occurred during printing.");
    MessageBoxA(param_1,local_10c,"Printing Error",0);
    break;
  case 0x28:
    FUN_0043ed39(local_10c,
                 (byte *)"The diagram is too large to be drawn in this version of Windows.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x29:
    FUN_0043ed39(local_10c,(byte *)"Internal diagram error.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x2a:
    FUN_0043ed39(local_10c,(byte *)"An equation must be terminated with a semicolon.");
    MessageBoxA(param_1,local_10c,"Syntax Error",0);
    break;
  case 0x2b:
    FUN_0043ed39(local_10c,
                 (byte *)
                 "Could not open the file. It may be in use by another application. Error code: %d")
    ;
    MessageBoxA(param_1,local_10c,"File Error",0);
    break;
  case 0x2c:
    FUN_0043ed39(local_10c,
                 (byte *)
                 "Could not open the file. It was created with a later version of Logic Friday and is incompatible with this version."
                );
    MessageBoxA(param_1,local_10c,"File Error",0);
    break;
  case 0x2d:
    FUN_0043ed39(local_10c,(byte *)"Length exceeds buffer size. You are limited to %dK characters");
    MessageBoxA(param_1,local_10c,"Equation Error",0);
    break;
  case 0x2e:
    FUN_0043ed39(local_10c,
                 (byte *)
                 "File error %d. Logic Friday could not create a necessary file and must close.");
    MessageBoxA(param_1,local_10c,"Error",0);
    break;
  case 0x2f:
    FUN_0043ed39(local_10c,
                 (byte *)
                 "File Error %d. The operation could not be completed because a Logic Friday\ndata file could not be opened. It may be in use by another application."
                );
    MessageBoxA(param_1,local_10c,"Error",0);
  }
  return;
}
