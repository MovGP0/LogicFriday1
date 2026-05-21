/* 00415dd9 FUN_00415dd9 */

undefined4 __thiscall
FUN_00415dd9(void *this,undefined4 param_1,undefined4 param_2,undefined4 *param_3)

{
  int iVar1;
  int iVar2;
  INT_PTR IVar3;
  int local_c;
  int local_8;
  
  local_8 = 0;
  *(undefined4 *)((int)this + 0x218) = 0;
  *(undefined4 *)((int)this + 0x214) = 0;
  *(undefined4 *)((int)this + 8) = param_1;
  *(undefined4 *)((int)this + 0xc) = param_2;
  iVar2 = FUN_004160ec((int)this);
  if (iVar2 == 0) {
    MessageBoxA(*(HWND *)this,
                "The two functions must have the same number of inputs and only one output.",
                "Compare Functions",0);
  }
  else {
    iVar2 = FUN_00416134((int)this);
    if (iVar2 == 2) {
      iVar2 = MessageBoxA(*(HWND *)this,
                          "The two functions have different input variable names. Do you want to compare on the basis of the\ntruth table ordering of input variables?"
                          ,"Compare Functions",3);
      if (iVar2 != 6) {
        return 0;
      }
      *(undefined4 *)((int)this + 0x218) = 1;
    }
    else if (iVar2 == 1) {
      IVar3 = DialogBoxParamA(*(HINSTANCE *)((int)this + 4),"NAMESORDERDLG",*(HWND *)this,
                              FUN_0040af4f,0);
      if (IVar3 == 0x40b) {
        local_8 = 1;
      }
      else {
        if (IVar3 != 0x40c) {
          return 0;
        }
        *(undefined4 *)((int)this + 0x218) = 1;
      }
    }
    FUN_004161ea(this,local_8);
    *(undefined4 *)((int)this + 0x20c) = 0;
    *(undefined4 *)((int)this + 0x208) = 0;
    *(undefined4 *)((int)this + 0x204) = 0;
    *(undefined4 *)((int)this + 0x214) = 0;
    for (local_c = 0; local_c < *(int *)((int)this + 0x10); local_c = local_c + 1) {
      iVar2 = *(int *)(*(int *)(*(int *)((int)this + 8) + 0x84) + local_c * 4);
      iVar1 = *(int *)(*(int *)((int)this + 0x94) + local_c * 4);
      if ((iVar2 == 1) && (iVar1 != 0)) {
        *(int *)((int)this + 0x204) = *(int *)((int)this + 0x204) + 1;
      }
      else if ((iVar1 == 1) && (iVar2 != 0)) {
        *(int *)((int)this + 0x204) = *(int *)((int)this + 0x204) + 1;
      }
      else if ((iVar2 == 1) && (iVar1 == 0)) {
        *(int *)((int)this + 0x208) = *(int *)((int)this + 0x208) + 1;
      }
      else if ((iVar2 == 0) && (iVar1 == 1)) {
        *(int *)((int)this + 0x20c) = *(int *)((int)this + 0x20c) + 1;
      }
    }
    if ((*(int *)((int)this + 0x208) == 0) && (*(int *)((int)this + 0x20c) == 0)) {
      *(undefined4 *)((int)this + 0x200) = 3;
    }
    else if ((*(int *)((int)this + 0x208) == 0) || (*(int *)((int)this + 0x20c) != 0)) {
      if ((*(int *)((int)this + 0x208) == 0) && (*(int *)((int)this + 0x20c) != 0)) {
        *(undefined4 *)((int)this + 0x200) = 5;
        *(undefined4 *)((int)this + 0x210) = 1;
      }
      else if (*(int *)((int)this + 0x204) == 0) {
        if (*(int *)((int)this + 0x208) + *(int *)((int)this + 0x20c) == *(int *)((int)this + 0x10))
        {
          *(undefined4 *)((int)this + 0x200) = 6;
        }
        else {
          *(undefined4 *)((int)this + 0x200) = 4;
        }
      }
      else {
        *(undefined4 *)((int)this + 0x200) = 4;
      }
    }
    else {
      *(undefined4 *)((int)this + 0x200) = 5;
      *(undefined4 *)((int)this + 0x210) = 2;
    }
    IVar3 = DialogBoxParamA(*(HINSTANCE *)((int)this + 4),"COMPRESDLG",*(HWND *)this,FUN_0040af6c,0)
    ;
    if (IVar3 != 0) {
      *param_3 = 1;
    }
  }
  return 0;
}
