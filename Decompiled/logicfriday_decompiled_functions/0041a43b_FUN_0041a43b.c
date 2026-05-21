/* 0041a43b FUN_0041a43b */

undefined4 __fastcall FUN_0041a43b(int param_1)

{
  LPVOID pvVar1;
  BOOL BVar2;
  
  pvVar1 = GlobalLock(*(HGLOBAL *)(param_1 + 0x68));
  *(uint *)((int)pvVar1 + 0x28) = *(uint *)((int)pvVar1 + 0x28) | 1;
  if (*(int *)(param_1 + 0xc0) == 1) {
    *(undefined2 *)((int)pvVar1 + 0x2c) = *(undefined2 *)(param_1 + 0x48);
  }
  else {
    *(undefined2 *)((int)pvVar1 + 0x2c) = *(undefined2 *)(param_1 + 0x4c);
  }
  GlobalUnlock(*(HGLOBAL *)(param_1 + 0x68));
  if (*(int *)(param_1 + 0xc0) == 1) {
    *(undefined4 *)(param_1 + 0x8c) = *(undefined4 *)(param_1 + 0x28);
    *(undefined4 *)(param_1 + 0x90) = *(undefined4 *)(param_1 + 0x2c);
    *(undefined4 *)(param_1 + 0x94) = *(undefined4 *)(param_1 + 0x30);
    *(undefined4 *)(param_1 + 0x98) = *(undefined4 *)(param_1 + 0x34);
  }
  else {
    *(undefined4 *)(param_1 + 0x8c) = *(undefined4 *)(param_1 + 0x38);
    *(undefined4 *)(param_1 + 0x90) = *(undefined4 *)(param_1 + 0x3c);
    *(undefined4 *)(param_1 + 0x94) = *(undefined4 *)(param_1 + 0x40);
    *(undefined4 *)(param_1 + 0x98) = *(undefined4 *)(param_1 + 0x44);
  }
  *(undefined4 *)(param_1 + 0x68) = *(undefined4 *)(param_1 + 0x10);
  *(undefined4 *)(param_1 + 0x6c) = *(undefined4 *)(param_1 + 0x14);
  *(undefined4 *)(param_1 + 0x70) = 0x26;
  BVar2 = PageSetupDlgA((LPPAGESETUPDLGA)(param_1 + 0x60));
  if (BVar2 != 0) {
    *(undefined4 *)(param_1 + 0x10) = *(undefined4 *)(param_1 + 0x68);
    *(undefined4 *)(param_1 + 0x14) = *(undefined4 *)(param_1 + 0x6c);
    pvVar1 = GlobalLock(*(HGLOBAL *)(param_1 + 0x68));
    *(uint *)((int)pvVar1 + 0x28) = *(uint *)((int)pvVar1 + 0x28) | 1;
    if (*(int *)(param_1 + 0xc0) == 1) {
      *(int *)(param_1 + 0x48) = (int)*(short *)((int)pvVar1 + 0x2c);
    }
    else {
      *(int *)(param_1 + 0x4c) = (int)*(short *)((int)pvVar1 + 0x2c);
    }
    GlobalUnlock(*(HGLOBAL *)(param_1 + 0x68));
    if (*(int *)(param_1 + 0xc0) == 1) {
      *(undefined4 *)(param_1 + 0x28) = *(undefined4 *)(param_1 + 0x8c);
      *(undefined4 *)(param_1 + 0x2c) = *(undefined4 *)(param_1 + 0x90);
      *(undefined4 *)(param_1 + 0x30) = *(undefined4 *)(param_1 + 0x94);
      *(undefined4 *)(param_1 + 0x34) = *(undefined4 *)(param_1 + 0x98);
    }
    else {
      *(undefined4 *)(param_1 + 0x38) = *(undefined4 *)(param_1 + 0x8c);
      *(undefined4 *)(param_1 + 0x3c) = *(undefined4 *)(param_1 + 0x90);
      *(undefined4 *)(param_1 + 0x40) = *(undefined4 *)(param_1 + 0x94);
      *(undefined4 *)(param_1 + 0x44) = *(undefined4 *)(param_1 + 0x98);
    }
  }
  return 0;
}
