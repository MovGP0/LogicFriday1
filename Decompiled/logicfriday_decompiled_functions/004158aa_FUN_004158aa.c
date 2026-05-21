/* 004158aa FUN_004158aa */

undefined4 __fastcall FUN_004158aa(int param_1)

{
  undefined1 local_18 [20];
  
  if (((*(int *)(param_1 + 0xda4) != 0) && (DAT_00452efc != 0)) && (DAT_00452efc != 1)) {
    InitializeSecurityDescriptor(local_18,1);
    SetSecurityDescriptorDacl(local_18,1,(PACL)0x0,0);
    SetSecurityDescriptorGroup(local_18,(PSID)0x0,0);
    SetSecurityDescriptorSacl(local_18,0,(PACL)0x0,0);
    SetFileSecurityA((LPCSTR)(param_1 + 0x784),4,local_18);
    SetFileSecurityA((LPCSTR)(param_1 + 0x888),4,local_18);
    SetFileSecurityA((LPCSTR)(param_1 + 0xc98),4,local_18);
  }
  return 1;
}
