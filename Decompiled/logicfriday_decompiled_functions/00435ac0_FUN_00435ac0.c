/* 00435ac0 FUN_00435ac0 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void __thiscall FUN_00435ac0(void *this,uint param_1)

{
  uint unaff_retaddr;
  char local_10c [260];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  switch(param_1 >> 0x10) {
  case 0x3e9:
    FUN_0043ed39(local_10c,(byte *)"Gate %s: Missing output connection.");
    MessageBoxA(*(HWND *)((int)this + 0x16f0),local_10c,"Diagram Error",0);
    break;
  case 0x3ea:
    FUN_0043ed39(local_10c,(byte *)"Gate %s: Missing input connection.");
    MessageBoxA(*(HWND *)((int)this + 0x16f0),local_10c,"Diagram Error",0);
    break;
  case 0x3eb:
    FUN_0043ed39(local_10c,(byte *)"Gate %s: Output is connected to another output.");
    MessageBoxA(*(HWND *)((int)this + 0x16f0),local_10c,"Diagram Error",0);
    break;
  case 0x3ec:
    FUN_0043ed39(local_10c,(byte *)"Gate %s: An input has no driving connection.");
    MessageBoxA(*(HWND *)((int)this + 0x16f0),local_10c,"Diagram Error",0);
    break;
  case 0x3ed:
    FUN_0043ed39(local_10c,(byte *)"The function must have at least two inputs.");
    MessageBoxA(*(HWND *)((int)this + 0x16f0),local_10c,"Diagram Error",0);
    break;
  case 0x3ee:
    FUN_0043ed39(local_10c,(byte *)"The number of inputs is limited to %d.");
    MessageBoxA(*(HWND *)((int)this + 0x16f0),local_10c,"Diagram Error",0);
    break;
  case 0x3ef:
    FUN_0043ed39(local_10c,(byte *)"The function must have at least one output.");
    MessageBoxA(*(HWND *)((int)this + 0x16f0),local_10c,"Diagram Error",0);
    break;
  case 0x3f0:
    FUN_0043ed39(local_10c,(byte *)"The number of outputs is limited to %d.");
    MessageBoxA(*(HWND *)((int)this + 0x16f0),local_10c,"Diagram Error",0);
    break;
  case 0x3f1:
    FUN_0043ed39(local_10c,(byte *)"Input terminal %s is not connected.");
    MessageBoxA(*(HWND *)((int)this + 0x16f0),local_10c,"Diagram Error",0);
    break;
  case 0x3f2:
    FUN_0043ed39(local_10c,
                 (byte *)
                 "Gate %s: An input is a function of the gate\'s output. Feedback is not supported."
                );
    MessageBoxA(*(HWND *)((int)this + 0x16f0),local_10c,"Diagram Error",0);
    break;
  default:
    FUN_0043ed39(local_10c,(byte *)"Unknown error: %d, %d");
    MessageBoxA(*(HWND *)((int)this + 0x16f0),local_10c,"Diagram Error",0);
  }
  return;
}
