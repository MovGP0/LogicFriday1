/* 0043a8f1 FUN_0043a8f1 */

undefined4 __thiscall
FUN_0043a8f1(void *this,undefined4 param_1,undefined4 param_2,uint param_3,undefined4 param_4)

{
  HCURSOR pHVar1;
  int local_c;
  int local_8;
  
  local_c = (int)(short)param_4;
  local_8 = (int)(short)((uint)param_4 >> 0x10);
  if (*(int *)((int)this + 0x44) == 0) {
    if ((local_8 < *(int *)((int)this + 0x28)) ||
       (*(int *)((int)this + 0x28) + *(int *)((int)this + 0x34) < local_8)) {
      if ((local_c < *(int *)((int)this + 0x2c)) ||
         (*(int *)((int)this + 0x2c) + *(int *)((int)this + 0x34) < local_c)) {
        if (((DAT_00452ef0 != 0) && (*(int *)((int)this + 0x30) <= local_8)) &&
           (local_8 <= *(int *)((int)this + 0x30) + *(int *)((int)this + 0x34))) {
          *(undefined4 *)((int)this + 0x6c) = 2;
        }
      }
      else {
        *(undefined4 *)((int)this + 0x6c) = 1;
      }
    }
    else {
      *(undefined4 *)((int)this + 0x6c) = 0;
    }
    if (*(int *)((int)this + 0x6c) == 1) {
      pHVar1 = LoadCursorA(DAT_00452914,"SPLITVERT");
      SetCursor(pHVar1);
    }
    else {
      pHVar1 = LoadCursorA(DAT_00452914,"SPLITHORZ");
      SetCursor(pHVar1);
    }
  }
  else {
    FUN_0043a9ff(this,&local_c);
    FUN_0043a5a9(this,local_c,local_8,param_3);
  }
  return 0;
}
