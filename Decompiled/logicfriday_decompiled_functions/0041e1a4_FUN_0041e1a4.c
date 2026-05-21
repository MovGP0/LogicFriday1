/* 0041e1a4 FUN_0041e1a4 */

/* WARNING: Function: __chkstk replaced with injection: alloca_probe */
/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_0041e1a4(void *this,undefined4 *param_1,char *param_2)

{
  undefined4 uVar1;
  char *pcVar2;
  size_t sVar3;
  void *pvVar4;
  uint unaff_retaddr;
  char local_841c [1024];
  fpos_t local_801c;
  uint local_8014 [8192];
  uint local_14;
  uint *local_10;
  uint local_c;
  char *local_8;
  
  local_8 = (char *)0x41e1b1;
  local_14 = DAT_00451a00 ^ unaff_retaddr;
  uVar1 = FUN_0043e6f2(param_2,"rt");
  *(undefined4 *)((int)this + 0x16d4) = uVar1;
  if (*(int *)((int)this + 0x16d4) == 0) {
    uVar1 = 0x2f000f;
  }
  else {
    FUN_0043ebd0(*(uint **)((int)this + 0x268),(uint *)"Factored:\n");
    for (local_c = 0; local_c < *(uint *)((int)this + 200); local_c = local_c + 1) {
      FUN_0043f99d(local_841c,0x400,*(FILE **)((int)this + 0x16d4));
      local_8 = _strchr(local_841c,0x7b);
      if (local_8 == (char *)0x0) {
        return 0x240001;
      }
      local_10 = local_8014;
      for (; *local_8 != '\0'; local_8 = local_8 + 1) {
        if ((((*local_8 != '{') && (*local_8 != '}')) && (*local_8 != '\n')) &&
           ((*local_8 != '\r' && (*local_8 != '\t')))) {
          *(char *)local_10 = *local_8;
          local_10 = (uint *)((int)local_10 + 1);
        }
      }
      _fgetpos(*(FILE **)((int)this + 0x16d4),&local_801c);
      while (pcVar2 = FUN_0043f99d(local_841c,0x400,*(FILE **)((int)this + 0x16d4)),
            pcVar2 != (char *)0x0) {
        local_8 = local_841c;
        pcVar2 = _strchr(local_8,0x7b);
        if (pcVar2 != (char *)0x0) {
          _fsetpos(*(FILE **)((int)this + 0x16d4),&local_801c);
          break;
        }
        for (; *local_8 != '\0'; local_8 = local_8 + 1) {
          if (((*local_8 != '}') && (*local_8 != '\t')) &&
             ((*local_8 != '\n' && (*local_8 != '\r')))) {
            *(char *)local_10 = *local_8;
            local_10 = (uint *)((int)local_10 + 1);
          }
        }
        _fgetpos(*(FILE **)((int)this + 0x16d4),&local_801c);
      }
      *(char *)local_10 = ';';
      *(char *)((int)local_10 + 1) = '\0';
      sVar3 = _strlen(*(char **)((int)this + 0x268));
      if (*(int *)((int)this + 0x165c) * 0x7fff - 0x100U < sVar3) {
        *(int *)((int)this + 0x165c) = *(int *)((int)this + 0x165c) + 1;
        pvVar4 = _realloc(*(void **)((int)this + 0x268),*(int *)((int)this + 0x165c) * 0x7fff);
        *(void **)((int)this + 0x268) = pvVar4;
      }
      FUN_0043ebe0(*(uint **)((int)this + 0x268),local_8014);
      FUN_0043ebe0(*(uint **)((int)this + 0x268),(uint *)&DAT_0044b734);
    }
    _fclose(*(FILE **)((int)this + 0x16d4));
    FUN_0043ebe0(*(uint **)((int)this + 0x268),(uint *)&DAT_0044b734);
    FUN_004219f6(this,*(uint **)((int)this + 0x268));
    *param_1 = *(undefined4 *)((int)this + 0x268);
    uVar1 = 0;
  }
  return uVar1;
}
